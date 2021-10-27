use super::mesh::{Mesh, Vertex};
use super::VulkanInfo;
use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;
use std::ffi::c_void;

pub struct Buffer {
    pub buffer: vk::Buffer,
    device_memory: vk::DeviceMemory,
    suballocations: Vec<SuballocatedBuffer>,
}

pub struct SuballocatedBuffer {
    offset: vk::DeviceSize,
    size: vk::DeviceSize,
}

impl Buffer {
    pub fn create_suballocated_buffer(&mut self, size: vk::DeviceSize) -> usize {
        if let Some(previous_suballocation) = self.suballocations.last() {
            let offset = previous_suballocation.offset + previous_suballocation.size;
            self.suballocations
                .push(SuballocatedBuffer { offset, size });
            self.suballocations.len() - 1
        } else {
            self.suballocations
                .push(SuballocatedBuffer { offset: 0, size });
            0
        }
    }
}

pub struct VertexBuffer {
    suballocation_index: usize,
    index_offset: vk::DeviceSize,
    vertex_offset: vk::DeviceSize,
    pub index_count: u32,
}

pub struct Image {
    pub image: vk::Image,
    device_memory: vk::DeviceMemory,
}

pub struct Allocator {
    staging_buffer: Buffer,
    staging_buffer_ptr: *mut c_void,
    vertex_buffer: Buffer,
    vertex_buffer_alignment: vk::DeviceSize,
    transfer_command_pool: vk::CommandPool,
    transfer_command_buffer: vk::CommandBuffer,
    transfer_command_fence: vk::Fence,
}

unsafe impl Send for Allocator {}

const TRANSFER_BUFFER_SIZE: vk::DeviceSize = 1_000_000; // 1 MB
const VERTEX_BUFFER_SIZE: vk::DeviceSize = 64_000_000; // 64 MB

impl Allocator {
    pub fn new(vulkan: &VulkanInfo) -> Self {
        // staging buffer

        let staging_buffer_create_info = vk::BufferCreateInfo::builder()
            .size(TRANSFER_BUFFER_SIZE)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC);

        let staging_buffer = allocate_buffer(
            vulkan,
            &staging_buffer_create_info,
            vk::MemoryPropertyFlags::HOST_VISIBLE,
        );

        let staging_buffer_ptr = unsafe {
            vulkan
                .device
                .map_memory(
                    staging_buffer.device_memory,
                    0,
                    vk::WHOLE_SIZE,
                    vk::MemoryMapFlags::empty(),
                )
                .expect("Vulkan: Unable to map memory.")
        };

        // device-local vertex buffer

        let vertex_buffer_create_info = vk::BufferCreateInfo::builder()
            .size(VERTEX_BUFFER_SIZE)
            .usage(
                vk::BufferUsageFlags::TRANSFER_DST
                    | vk::BufferUsageFlags::VERTEX_BUFFER
                    | vk::BufferUsageFlags::INDEX_BUFFER,
            );

        let vertex_buffer = allocate_buffer(
            vulkan,
            &vertex_buffer_create_info,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        let vertex_buffer_memory_requirements = unsafe {
            vulkan
                .device
                .get_buffer_memory_requirements(vertex_buffer.buffer)
        };
        let vertex_buffer_alignment = vertex_buffer_memory_requirements.alignment;
        assert!(
            vertex_buffer_alignment & (vertex_buffer_alignment - 1) == 0,
            "Vulkan: Invalid vertex buffer alignment."
        );

        // transfer command buffer and sync

        let transfer_command_pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::TRANSIENT)
            .queue_family_index(vulkan.device_queues.transfer_queue.family_index);

        let transfer_command_pool = unsafe {
            vulkan
                .device
                .create_command_pool(&transfer_command_pool_create_info, None)
                .expect("Vulkan: Unable to create transfer command pool.")
        };

        let transfer_command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(transfer_command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let transfer_command_buffer = unsafe {
            vulkan
                .device
                .allocate_command_buffers(&transfer_command_buffer_allocate_info)
                .expect("Vulkan: Unable to allocate transfer command buffer.")
                .into_iter()
                .last()
                .unwrap()
        };

        let transfer_fence_create_info =
            vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

        let transfer_command_fence = unsafe {
            vulkan
                .device
                .create_fence(&transfer_fence_create_info, None)
                .expect("Vulkan: Unable to create transfer command fence.")
        };

        Allocator {
            staging_buffer,
            staging_buffer_ptr,
            vertex_buffer,
            vertex_buffer_alignment,
            transfer_command_pool,
            transfer_command_buffer,
            transfer_command_fence,
        }
    }

    pub unsafe fn destroy(&self, vulkan: &VulkanInfo) {
        vulkan
            .device
            .destroy_fence(self.transfer_command_fence, None);

        vulkan
            .device
            .free_command_buffers(self.transfer_command_pool, &[self.transfer_command_buffer]);

        vulkan
            .device
            .destroy_command_pool(self.transfer_command_pool, None);

        free_buffer_unsafe(vulkan, &self.vertex_buffer);

        vulkan
            .device
            .unmap_memory(self.staging_buffer.device_memory);

        free_buffer_unsafe(vulkan, &self.staging_buffer);
    }

    pub fn transfer_vertex_buffer(&mut self, vulkan: &VulkanInfo, mesh: &Mesh) -> VertexBuffer {
        let indices_size =
            self.aligned_size(std::mem::size_of_val(mesh.indices.as_slice()) as vk::DeviceSize);

        let vertices_size =
            self.aligned_size(std::mem::size_of_val(mesh.vertices.as_slice()) as vk::DeviceSize);

        assert!(
            indices_size + vertices_size <= TRANSFER_BUFFER_SIZE,
            "Vulkan: vertex buffer too large for staging buffer."
        );

        // ensure staging buffer is not in use by previous transfer operation

        unsafe {
            vulkan // wait_for_fences
                .device
                .wait_for_fences(&[self.transfer_command_fence], false, u64::MAX)
                .unwrap();

            vulkan // reset_fences
                .device
                .reset_fences(&[self.transfer_command_fence])
                .unwrap();
        }

        // copy vertex data to staging buffer

        unsafe {
            let index_buffer_ptr = self.staging_buffer_ptr as *mut u16;
            let index_buffer_slice =
                std::slice::from_raw_parts_mut(index_buffer_ptr, mesh.indices.len());
            index_buffer_slice.copy_from_slice(&mesh.indices);
        };

        unsafe {
            let vertex_buffer_ptr_raw = self.staging_buffer_ptr as *mut u8;
            let vertex_buffer_ptr =
                vertex_buffer_ptr_raw.offset(indices_size as isize) as *mut Vertex;
            let vertex_buffer_slice =
                std::slice::from_raw_parts_mut(vertex_buffer_ptr, mesh.vertices.len());
            vertex_buffer_slice.copy_from_slice(&mesh.vertices);
        };

        // create suballocated buffer

        let suballocation_index = self
            .vertex_buffer
            .create_suballocated_buffer(vertices_size + indices_size);

        // transfer

        unsafe {
            self.flush_and_transfer_staged_data(vulkan, &self.vertex_buffer, suballocation_index);
        }

        // return suballocated buffer

        VertexBuffer {
            suballocation_index,
            index_offset: 0,
            vertex_offset: indices_size,
            index_count: mesh.indices.len() as u32,
        }
    }

    fn aligned_size(&self, size: vk::DeviceSize) -> vk::DeviceSize {
        let alignment = self.vertex_buffer_alignment;
        (size + alignment - 1) / alignment * alignment
    }

    /// Flushes the host-visible transfer buffer and transfers the suballocated buffer
    unsafe fn flush_and_transfer_staged_data(
        &self,
        vulkan: &VulkanInfo,
        buffer: &Buffer,
        suballocation_index: usize,
    ) {
        let suballocated_buffer = &buffer.suballocations[suballocation_index];

        // flush staging buffer

        let memory_ranges = [vk::MappedMemoryRange::builder()
            .memory(self.staging_buffer.device_memory)
            .offset(0)
            .size(suballocated_buffer.size)
            .build()];

        vulkan
            .device
            .flush_mapped_memory_ranges(&memory_ranges)
            .unwrap();

        // write transfer command buffer

        vulkan // reset_command_pool
            .device
            .reset_command_pool(
                self.transfer_command_pool,
                vk::CommandPoolResetFlags::empty(),
            )
            .unwrap();

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        vulkan // begin_command_buffer
            .device
            .begin_command_buffer(self.transfer_command_buffer, &command_buffer_begin_info)
            .unwrap();

        let buffer_copy_region = vk::BufferCopy::builder()
            .src_offset(0)
            .dst_offset(suballocated_buffer.offset)
            .size(suballocated_buffer.size);

        vulkan.device.cmd_copy_buffer(
            self.transfer_command_buffer,
            self.staging_buffer.buffer,
            self.vertex_buffer.buffer,
            &[buffer_copy_region.build()],
        );

        vulkan // end_command_buffer
            .device
            .end_command_buffer(self.transfer_command_buffer)
            .unwrap();

        let command_buffers = [self.transfer_command_buffer];

        let submit_infos = [vk::SubmitInfo::builder()
            .command_buffers(&command_buffers)
            .build()];

        // TODO: efficient sync
        vulkan.device.device_wait_idle().unwrap();

        vulkan // queue_submit
            .device
            .queue_submit(
                vulkan.device_queues.transfer_queue.queue,
                &submit_infos,
                self.transfer_command_fence,
            )
            .unwrap();
    }

    pub fn free_vertex_buffer(&mut self, vertex_buffer: VertexBuffer) {
        debug_assert!(
            vertex_buffer.suballocation_index == self.vertex_buffer.suballocations.len() - 1
        );

        self.vertex_buffer.suballocations.pop();
    }

    pub fn bind_vertex_buffer(
        &self,
        vulkan: &VulkanInfo,
        command_buffer: vk::CommandBuffer,
        vertex_buffer: &VertexBuffer,
    ) {
        let buffers = [self.vertex_buffer.buffer];
        let offsets = [vertex_buffer.vertex_offset];

        unsafe {
            vulkan
                .device
                .cmd_bind_vertex_buffers(command_buffer, 0, &buffers, &offsets);

            vulkan.device.cmd_bind_index_buffer(
                command_buffer,
                self.vertex_buffer.buffer,
                vertex_buffer.index_offset,
                vk::IndexType::UINT16,
            );
        }
    }
}

pub fn allocate_buffer(
    vulkan: &VulkanInfo,
    buffer_create_info: &vk::BufferCreateInfo,
    memory_properties: vk::MemoryPropertyFlags,
) -> Buffer {
    let buffer = unsafe {
        vulkan
            .device
            .create_buffer(buffer_create_info, None)
            .expect("Vulkan: Unable to allocate buffer.")
    };

    let device_memory_properties = unsafe {
        vulkan
            .instance
            .get_physical_device_memory_properties(vulkan.physical_device)
    };
    let buffer_memory_requirements =
        unsafe { vulkan.device.get_buffer_memory_requirements(buffer) };
    let mut buffer_memory_type_index = None;
    for (i, memory_type) in device_memory_properties.memory_types.iter().enumerate() {
        if (memory_type.property_flags & memory_properties) == memory_properties
            && buffer_memory_requirements.memory_type_bits & (1 << i) != 0
        {
            buffer_memory_type_index = Some(i as u32);
            break;
        }
    }

    let memory_allocate_info = vk::MemoryAllocateInfo::builder()
        .allocation_size(buffer_memory_requirements.size)
        .memory_type_index(
            buffer_memory_type_index.expect("Vulkan: No suitable memory type found for buffer."),
        );

    let device_memory = unsafe {
        vulkan
            .device
            .allocate_memory(&memory_allocate_info, None)
            .expect("Vulkan: Could not allocate buffer memory.")
    };

    unsafe {
        vulkan
            .device
            .bind_buffer_memory(buffer, device_memory, 0)
            .expect("Vulkan: Unable to bind buffer memory.");
    }

    Buffer {
        buffer,
        device_memory,
        suballocations: Vec::new(),
    }
}

pub fn free_buffer(vulkan: &VulkanInfo, buffer: Buffer) {
    unsafe {
        vulkan.device.free_memory(buffer.device_memory, None);
        vulkan.device.destroy_buffer(buffer.buffer, None);
    }
}

/// Frees a buffer without consuming the buffer. free_buffer() should be used if possible.
pub unsafe fn free_buffer_unsafe(vulkan: &VulkanInfo, buffer: &Buffer) {
    vulkan.device.free_memory(buffer.device_memory, None);
    vulkan.device.destroy_buffer(buffer.buffer, None);
}

pub fn map(vulkan: &VulkanInfo, buffer: &Buffer) -> *mut c_void {
    unsafe {
        vulkan
            .device
            .map_memory(
                buffer.device_memory,
                0,
                vk::WHOLE_SIZE,
                vk::MemoryMapFlags::empty(),
            )
            .expect("Vulkan: Unable to map memory.")
    }
}

pub fn unmap(vulkan: &VulkanInfo, buffer: &Buffer) {
    unsafe {
        vulkan.device.unmap_memory(buffer.device_memory);
    }
}

pub fn allocate_image(
    vulkan: &VulkanInfo,
    image_create_info: &vk::ImageCreateInfo,
    memory_properties: vk::MemoryPropertyFlags,
) -> Image {
    let image = unsafe {
        vulkan
            .device
            .create_image(image_create_info, None)
            .expect("Vulkan: Unable to allocate image.")
    };

    let device_memory_properties = unsafe {
        vulkan
            .instance
            .get_physical_device_memory_properties(vulkan.physical_device)
    };
    let image_memory_requirements = unsafe { vulkan.device.get_image_memory_requirements(image) };
    let mut image_memory_type_index = None;
    for (i, memory_type) in device_memory_properties.memory_types.iter().enumerate() {
        if (memory_type.property_flags & memory_properties) == memory_properties
            && image_memory_requirements.memory_type_bits & (1 << i) != 0
        {
            image_memory_type_index = Some(i as u32);
            break;
        }
    }

    let memory_allocate_info = vk::MemoryAllocateInfo::builder()
        .allocation_size(image_memory_requirements.size)
        .memory_type_index(
            image_memory_type_index.expect("Vulkan: No suitable memory type found for image."),
        );

    let device_memory = unsafe {
        vulkan
            .device
            .allocate_memory(&memory_allocate_info, None)
            .expect("Vulkan: Could not allocate image memory.")
    };

    unsafe {
        vulkan
            .device
            .bind_image_memory(image, device_memory, 0)
            .expect("Vulkan: Unable to bind buffer memory.")
    };

    Image {
        image,
        device_memory,
    }
}

fn free_image(vulkan: &VulkanInfo, image: Image) {
    unsafe {
        vulkan.device.free_memory(image.device_memory, None);
        vulkan.device.destroy_image(image.image, None);
    }
}

/// Frees an image without consuming the image. free_image() should be used if possible.
pub unsafe fn free_image_unsafe(vulkan: &VulkanInfo, image: &Image) {
    vulkan.device.free_memory(image.device_memory, None);
    vulkan.device.destroy_image(image.image, None);
}
