use super::allocator;
use super::VulkanInfo;
use ash::{version::DeviceV1_0, version::InstanceV1_0, vk};
use nalgebra_glm::Mat4;
use std::ffi::c_void;

#[derive(Clone, Copy)]
pub struct InstanceData {
    pub model_matrix: Mat4,
}

/// CurrentFrameInfo does not implement Clone or Copy, providing safety
/// by ensuring that the command buffer is not accessed at an unexpected time
pub struct CurrentFrameInfo {
    pub command_buffer: vk::CommandBuffer,
    pub acquire_semaphore: vk::Semaphore,
    pub instance_descriptor_set: vk::DescriptorSet,
}

pub struct Frame {
    descriptor_pool: vk::DescriptorPool,
    command_pool: vk::CommandPool,
    command_fence: vk::Fence,
    command_buffer: vk::CommandBuffer,
    acquire_semaphore: vk::Semaphore,
    present_semaphore: vk::Semaphore,
    instance_data_alignment: vk::DeviceSize,
    instance_data_buffer: allocator::Buffer,
    instance_data_ptr: *mut c_void,
    instance_descriptor_set: vk::DescriptorSet,
}

unsafe impl Send for Frame {}

impl Frame {
    pub fn new(vulkan: &VulkanInfo) -> Self {
        // descriptor pool
        let descriptor_pool_sizes = [vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
            .descriptor_count(1)
            .build()];

        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::builder()
            .max_sets(1)
            .pool_sizes(&descriptor_pool_sizes);

        let descriptor_pool = unsafe {
            vulkan
                .device
                .create_descriptor_pool(&descriptor_pool_create_info, None)
                .expect("Vulkan: Failed to create frame descriptor pool.")
        };

        // command pool + sync

        let command_pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::TRANSIENT)
            .queue_family_index(vulkan.device_queues.graphics_queue.family_index);

        let command_pool = unsafe {
            vulkan
                .device
                .create_command_pool(&command_pool_create_info, None)
                .expect("Vulkan: Failed to create frame command pool.")
        };

        let command_fence_create_info =
            vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

        let command_fence = unsafe {
            vulkan
                .device
                .create_fence(&command_fence_create_info, None)
                .expect("Vulkan: Failed to create frame command buffer fence.")
        };

        // allocate command buffer

        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffer = unsafe {
            vulkan
                .device
                .allocate_command_buffers(&command_buffer_allocate_info)
                .expect("Vulkan: Failed to allocate frame command buffer.")
                .into_iter()
                .last()
                .unwrap()
        };

        // acquire + present semaphores

        let semaphore_create_info = vk::SemaphoreCreateInfo::builder();

        let acquire_semaphore = unsafe {
            vulkan
                .device
                .create_semaphore(&semaphore_create_info, None)
                .expect("Vulkan: Failed to create frame acquire semaphore.")
        };

        let present_semaphore = unsafe {
            vulkan
                .device
                .create_semaphore(&semaphore_create_info, None)
                .expect("Vulkan: Failed to create frame present semaphore.")
        };

        // allocate descriptor set

        let descriptor_set_layouts = [vulkan.descriptor_set_layouts.instance_layout];

        let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&descriptor_set_layouts);

        let instance_descriptor_set = unsafe {
            vulkan
                .device
                .allocate_descriptor_sets(&descriptor_set_allocate_info)
                .expect("Vulkan: Failed to create frame descriptor set.")
                .into_iter()
                .last()
                .unwrap()
        };

        // allocate uniform buffer memory

        let min_ubo_alignment = unsafe {
            vulkan
                .instance
                .get_physical_device_properties(vulkan.physical_device)
                .limits
                .min_uniform_buffer_offset_alignment
        };

        let instance_data_alignment =
            (std::mem::size_of::<InstanceData>() as vk::DeviceSize + min_ubo_alignment - 1)
                & !(min_ubo_alignment - 1);

        let buffer_create_info = vk::BufferCreateInfo::builder()
            .size(instance_data_alignment * 4) // TEMPORARY
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER);

        let memory_properties =
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
        let instance_data_buffer =
            allocator::allocate_buffer(vulkan, &buffer_create_info, memory_properties);
        let instance_data_ptr = allocator::map(vulkan, &instance_data_buffer);

        // associate uniform buffer memory with descirptor set

        let instance_descriptor_buffer_info = [vk::DescriptorBufferInfo::builder()
            .buffer(instance_data_buffer.buffer)
            .offset(0)
            .range(vk::WHOLE_SIZE)
            .build()];

        let instance_descriptor_set_writes = [vk::WriteDescriptorSet::builder()
            .dst_set(instance_descriptor_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
            .buffer_info(&instance_descriptor_buffer_info)
            .build()];

        unsafe {
            vulkan
                .device
                .update_descriptor_sets(&instance_descriptor_set_writes, &[]);
        }

        Frame {
            descriptor_pool,
            command_pool,
            command_fence,
            command_buffer,
            acquire_semaphore,
            present_semaphore,
            instance_data_alignment,
            instance_data_buffer,
            instance_data_ptr,
            instance_descriptor_set,
        }
    }

    pub unsafe fn destroy(&self, vulkan: &VulkanInfo) {
        allocator::unmap(vulkan, &self.instance_data_buffer);
        allocator::free_buffer_unsafe(vulkan, &self.instance_data_buffer);
        vulkan
            .device
            .destroy_semaphore(self.acquire_semaphore, None);
        vulkan
            .device
            .destroy_semaphore(self.present_semaphore, None);
        vulkan.device.destroy_fence(self.command_fence, None);
        vulkan.device.destroy_command_pool(self.command_pool, None);
        vulkan
            .device
            .destroy_descriptor_pool(self.descriptor_pool, None);
    }

    pub fn begin(&self, vulkan: &VulkanInfo) -> CurrentFrameInfo {
        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            vulkan
                .device
                .wait_for_fences(&[self.command_fence], false, u64::MAX)
                .unwrap();
            vulkan.device.reset_fences(&[self.command_fence]).unwrap();
            vulkan
                .device
                .reset_command_pool(self.command_pool, vk::CommandPoolResetFlags::empty())
                .unwrap();
            vulkan
                .device
                .begin_command_buffer(self.command_buffer, &command_buffer_begin_info)
                .unwrap();
        }

        CurrentFrameInfo {
            command_buffer: self.command_buffer,
            acquire_semaphore: self.acquire_semaphore,
            instance_descriptor_set: self.instance_descriptor_set,
        }
    }

    pub fn update_instance(&self, instance_index: usize, instance_data: &InstanceData) {
        unsafe {
            let instance_data_ptr_raw = self.instance_data_ptr as *mut u8;
            let instance_data_offset_ptr = instance_data_ptr_raw
                .add(instance_index * self.instance_data_alignment as usize)
                as *mut InstanceData;
            *instance_data_offset_ptr = *instance_data;
        };
    }

    pub fn end_and_submit(
        &self,
        vulkan: &VulkanInfo,
        _current_frame_info: CurrentFrameInfo,
    ) -> vk::Semaphore {
        let submits_info = [vk::SubmitInfo::builder()
            .wait_semaphores(&[self.acquire_semaphore])
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::TOP_OF_PIPE])
            .command_buffers(&[self.command_buffer])
            .signal_semaphores(&[self.present_semaphore])
            .build()];

        unsafe {
            vulkan
                .device
                .end_command_buffer(self.command_buffer)
                .unwrap();

            vulkan
                .device
                .queue_submit(
                    vulkan.device_queues.graphics_queue.queue,
                    &submits_info,
                    self.command_fence,
                )
                .unwrap();
        }

        self.present_semaphore
    }
}
