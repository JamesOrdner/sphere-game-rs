mod allocator;
mod depth_buffer;
mod descriptor_set_layouts;
mod device;
mod frame;
mod framebuffers;
mod instance;
pub mod mesh;
mod pipeline;
mod pipeline_layouts;
mod render_pass;
mod swapchain;

use allocator::Allocator;
use ash::{extensions::khr, version::DeviceV1_0, vk};
use depth_buffer::DepthBuffer;
use descriptor_set_layouts::DescriptorSetLayouts;
use frame::Frame;
use mesh::Mesh;
use pipeline::Pipeline;
use pipeline_layouts::PipelineLayouts;
use swapchain::Swapchain;
use winit::window::Window;

pub use allocator::VertexBuffer;
pub use frame::InstanceData;
pub use pipeline_layouts::SceneData;

/// VulkanInfo contains constant data which will not be mutated during the lifetime of an instance
pub struct VulkanInfo {
    _window: Window,
    _entry: ash::Entry,
    instance: ash::Instance,
    surface_loader: khr::Surface,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    device_queues: device::Queues,
    descriptor_set_layouts: DescriptorSetLayouts,
    pipeline_layouts: PipelineLayouts,
}

struct CurrentFrameInfo {
    frame_info: frame::CurrentFrameInfo,
    swapchain_image_index: u32,
}

pub struct Vulkan {
    info: VulkanInfo,
    allocator: Allocator,
    swapchain: Swapchain,
    depth_buffer: DepthBuffer,
    render_pass: vk::RenderPass,
    framebuffers: Vec<vk::Framebuffer>,
    pipeline: Pipeline,
    frames: [Frame; 1],
    current_frame_index: usize,
    current_frame_info: Option<CurrentFrameInfo>,
}

impl Vulkan {
    pub fn new(window: Window) -> Self {
        let (entry, instance, surface_loader, surface) = instance::new(&window);
        let (physical_device, device, device_queues) =
            device::new(&instance, &surface_loader, surface);
        let descriptor_set_layouts = DescriptorSetLayouts::new(&device);
        let pipeline_layouts = PipelineLayouts::new(&device, &descriptor_set_layouts);

        let vulkan_info = VulkanInfo {
            _window: window,
            _entry: entry,
            instance,
            surface_loader,
            surface,
            physical_device,
            device,
            device_queues,
            descriptor_set_layouts,
            pipeline_layouts,
        };

        let allocator = Allocator::new(&vulkan_info);
        let swapchain = Swapchain::new(&vulkan_info);
        let depth_buffer = DepthBuffer::new(&vulkan_info, &swapchain);
        let render_pass = render_pass::new(&vulkan_info, &swapchain, &depth_buffer);
        let framebuffers = framebuffers::new(&vulkan_info, &depth_buffer, &swapchain, render_pass);
        let pipeline = Pipeline::new(&vulkan_info, &swapchain, render_pass, "default");
        let frames = [Frame::new(&vulkan_info)];

        Vulkan {
            info: vulkan_info,
            allocator,
            swapchain,
            depth_buffer,
            render_pass,
            framebuffers,
            pipeline,
            frames,
            current_frame_index: 0,
            current_frame_info: None,
        }
    }
}

impl Drop for Vulkan {
    fn drop(&mut self) {
        unsafe {
            self.info.device.device_wait_idle().unwrap();

            for frame in &self.frames {
                frame.destroy(&self.info);
            }
            self.pipeline.destroy(&self.info);
            framebuffers::destroy(&self.framebuffers, &self.info);
            render_pass::destroy(self.render_pass, &self.info);
            self.depth_buffer.destroy(&self.info);
            self.swapchain.destroy(&self.info);
            self.allocator.destroy(&self.info);
            self.info.pipeline_layouts.destroy(&self.info);
            self.info.descriptor_set_layouts.destroy(&self.info);
            device::destroy(&self.info);
            instance::destroy(&self.info);
        }
    }
}

impl Vulkan {
    pub fn load_mesh(&mut self, mesh: &Mesh) -> VertexBuffer {
        self.allocator.transfer_vertex_buffer(&self.info, mesh)
    }

    pub fn unload_last_mesh(&mut self, vertex_buffer: VertexBuffer) {
        self.allocator.free_vertex_buffer(vertex_buffer)
    }

    pub fn begin_instance_update(&mut self) {
        let frame_info = self.frames[self.current_frame_index].begin(&self.info);
        let swapchain_image_index = self
            .swapchain
            .acquire_next_image(frame_info.acquire_semaphore);

        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];

        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.render_pass)
            .framebuffer(self.framebuffers[swapchain_image_index as usize])
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain.surface_extent,
            })
            .clear_values(&clear_values);

        unsafe {
            self.info.device.cmd_begin_render_pass(
                frame_info.command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );
        }

        self.current_frame_info = Some(CurrentFrameInfo {
            frame_info,
            swapchain_image_index,
        });
    }

    pub fn update_scene(&self, scene_data: &SceneData) {
        let command_buffer = self
            .current_frame_info
            .as_ref()
            .unwrap()
            .frame_info
            .command_buffer;

        unsafe {
            let data = std::slice::from_raw_parts(
                (scene_data as *const SceneData) as *const u8,
                std::mem::size_of_val(scene_data),
            );

            self.info.device.cmd_push_constants(
                command_buffer,
                self.info.pipeline_layouts.scene_layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                data,
            );
        }
    }

    pub fn update_instance(&self, instance_index: usize, instance_data: &InstanceData) {
        self.frames[self.current_frame_index].update_instance(instance_index, instance_data);
    }

    pub fn draw_instance(&self, instance_index: usize, vertex_buffer: &VertexBuffer) {
        let frame_info = &self.current_frame_info.as_ref().unwrap().frame_info;
        let descriptor_sets = [frame_info.instance_descriptor_set];
        let dynamic_offsets = [instance_index as u32];

        self.pipeline.bind(&self.info, frame_info.command_buffer);

        unsafe {
            self.info.device.cmd_bind_descriptor_sets(
                frame_info.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.info.pipeline_layouts.scene_layout,
                0,
                &descriptor_sets,
                &dynamic_offsets,
            );
        }

        self.allocator
            .bind_vertex_buffer(&self.info, frame_info.command_buffer, vertex_buffer);

        unsafe {
            self.info.device.cmd_draw_indexed(
                frame_info.command_buffer,
                vertex_buffer.index_count,
                1,
                0,
                0,
                0,
            );
        }
    }

    pub fn end_instance_update_and_render(&mut self) {
        let current_frame_info = self.current_frame_info.take().unwrap();

        unsafe {
            self.info
                .device
                .cmd_end_render_pass(current_frame_info.frame_info.command_buffer);
        }

        let present_semaphore = self.frames[self.current_frame_index]
            .end_and_submit(&self.info, current_frame_info.frame_info);

        self.swapchain.present(
            &self.info,
            present_semaphore,
            current_frame_info.swapchain_image_index,
        );

        self.current_frame_index = (self.current_frame_index + 1) % self.frames.len();
    }
}
