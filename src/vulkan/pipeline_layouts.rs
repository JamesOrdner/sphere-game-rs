use super::descriptor_set_layouts::DescriptorSetLayouts;
use super::VulkanInfo;
use ash::{version::DeviceV1_0, vk};
use nalgebra_glm as glm;

pub struct SceneData {
    pub proj_matrix: glm::Mat4,
    pub view_matrix: glm::Mat4,
}

pub struct PipelineLayouts {
    pub scene_layout: vk::PipelineLayout,
}

impl PipelineLayouts {
    pub fn new(device: &ash::Device, descriptor_set_layouts: &DescriptorSetLayouts) -> Self {
        let descriptor_set_layouts = [descriptor_set_layouts.instance_layout];

        let push_constant_ranges = [vk::PushConstantRange::builder()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(std::mem::size_of::<SceneData>() as u32)
            .build()];

        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&descriptor_set_layouts)
            .push_constant_ranges(&push_constant_ranges);

        let scene_layout = unsafe {
            device
                .create_pipeline_layout(&pipeline_layout_create_info, None)
                .expect("Vulkan: Unable to create scene pipeline layout.")
        };

        PipelineLayouts { scene_layout }
    }

    pub unsafe fn destroy(&self, vulkan: &VulkanInfo) {
        vulkan
            .device
            .destroy_pipeline_layout(self.scene_layout, None);
    }
}
