use super::mesh::Vertex;
use super::swapchain::Swapchain;
use super::VulkanInfo;
use ash::{version::DeviceV1_0, vk};
use nalgebra_glm as glm;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::mem::size_of;
use std::path::{Path, PathBuf};

pub struct Pipeline {
    pipeline: vk::Pipeline,
}

impl Pipeline {
    pub fn new(
        vulkan: &VulkanInfo,
        swapchain: &Swapchain,
        render_pass: vk::RenderPass,
        shader_name: &str,
    ) -> Self {
        let shader_entry = CString::new("main").unwrap();
        let shader = Shader::new(vulkan, shader_name, &shader_entry);

        let vertex_input_binding_descriptions = [vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()];

        let vertex_input_attribue_descriptions = [
            vk::VertexInputAttributeDescription::builder()
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(0)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .location(1)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(size_of::<glm::Vec3>() as u32)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .location(2)
                .format(vk::Format::R32G32_SFLOAT)
                .offset((size_of::<glm::Vec3>() * 2) as u32)
                .build(),
        ];

        let vertex_input_create_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&vertex_input_binding_descriptions)
            .vertex_attribute_descriptions(&vertex_input_attribue_descriptions);

        let input_assembly_create_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

        let viewports = [vk::Viewport::builder()
            .width(swapchain.surface_extent.width as f32)
            .height(swapchain.surface_extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .build()];

        let scissors = [vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(swapchain.surface_extent)
            .build()];

        let viewport_create_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors);

        let rasterization_create_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .line_width(1.0);

        let multisample_create_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let depth_stencil_create_info = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);

        let color_blend_attachments = [vk::PipelineColorBlendAttachmentState::builder()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::all())
            .build()];

        let color_blend_create_info =
            vk::PipelineColorBlendStateCreateInfo::builder().attachments(&color_blend_attachments);

        let pipeline_create_info = [vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader.stages)
            .vertex_input_state(&vertex_input_create_info)
            .input_assembly_state(&input_assembly_create_info)
            .viewport_state(&viewport_create_info)
            .rasterization_state(&rasterization_create_info)
            .multisample_state(&multisample_create_info)
            .depth_stencil_state(&depth_stencil_create_info)
            .color_blend_state(&color_blend_create_info)
            .layout(vulkan.pipeline_layouts.scene_layout)
            .render_pass(render_pass)
            .subpass(0)
            .build()];

        let pipeline = unsafe {
            vulkan
                .device
                .create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_create_info, None)
                .expect("Vulkan: Failed to create pipeline.")
                .into_iter()
                .last()
                .unwrap()
        };

        Pipeline { pipeline }
    }

    pub unsafe fn destroy(&self, vulkan: &VulkanInfo) {
        vulkan.device.destroy_pipeline(self.pipeline, None);
    }

    pub fn bind(&self, vulkan: &VulkanInfo, command_buffer: vk::CommandBuffer) {
        unsafe {
            vulkan.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );
        }
    }
}

struct Shader<'a> {
    vulkan: &'a VulkanInfo,
    stages: Vec<vk::PipelineShaderStageCreateInfo>,
    vert_shader_module: vk::ShaderModule,
    frag_shader_module: vk::ShaderModule,
}

const SHADERS_DIR: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../res/shaders");

impl<'a> Shader<'a> {
    fn new(vulkan: &'a VulkanInfo, name: &str, entry: &'a CStr) -> Self {
        let path = Path::new(SHADERS_DIR).join(name);
        let mut stages = Vec::with_capacity(2);

        let vert_code = read_shader_file(&path.with_extension("vert.spv"))
            .expect("Vulkan: Failed to load vertex shader.");
        let vert_shader_module = unsafe { create_shader_module(vulkan, &vert_code) };
        let vert_shader_stage_create_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_shader_module)
            .name(entry)
            .build();
        stages.push(vert_shader_stage_create_info);

        let frag_code = read_shader_file(&path.with_extension("frag.spv"))
            .expect("Vulkan: Failed to load vertex shader.");
        let frag_shader_module = unsafe { create_shader_module(vulkan, &frag_code) };
        let frag_shader_stage_create_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_shader_module)
            .name(entry)
            .build();
        stages.push(frag_shader_stage_create_info);

        Shader {
            vulkan,
            stages,
            vert_shader_module,
            frag_shader_module,
        }
    }
}

impl<'a> Drop for Shader<'a> {
    fn drop(&mut self) {
        unsafe {
            self.vulkan
                .device
                .destroy_shader_module(self.vert_shader_module, None);
            self.vulkan
                .device
                .destroy_shader_module(self.frag_shader_module, None);
        }
    }
}

fn read_shader_file(path: &PathBuf) -> std::io::Result<Vec<u32>> {
    let mut file = File::open(path).expect("Vulkan: Failed to open shader file.");
    ash::util::read_spv(&mut file)
}

unsafe fn create_shader_module(vulkan: &VulkanInfo, code: &[u32]) -> vk::ShaderModule {
    let shader_module_create_info = vk::ShaderModuleCreateInfo::builder().code(code);
    vulkan
        .device
        .create_shader_module(&shader_module_create_info, None)
        .expect("Vulkan: Failed to create shader module.")
}
