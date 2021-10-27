use super::VulkanInfo;
use ash::{version::DeviceV1_0, vk};

pub struct DescriptorSetLayouts {
    pub instance_layout: vk::DescriptorSetLayout,
}

impl DescriptorSetLayouts {
    pub fn new(device: &ash::Device) -> Self {
        let instance_layout_binding = [vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .build()];

        let instance_layout_create_info =
            vk::DescriptorSetLayoutCreateInfo::builder().bindings(&instance_layout_binding);

        let instance_layout = unsafe {
            device
                .create_descriptor_set_layout(&instance_layout_create_info, None)
                .expect("Vulkan: Unable to create instance descriptor set layout.")
        };

        DescriptorSetLayouts { instance_layout }
    }

    pub unsafe fn destroy(&self, vulkan: &VulkanInfo) {
        vulkan
            .device
            .destroy_descriptor_set_layout(self.instance_layout, None);
    }
}
