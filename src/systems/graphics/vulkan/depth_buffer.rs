use super::allocator;
use super::swapchain::Swapchain;
use super::VulkanInfo;
use ash::{
    version::{DeviceV1_0, InstanceV1_0},
    vk,
};

pub struct DepthBuffer {
    pub format: vk::Format,
    image: allocator::Image,
    pub image_view: vk::ImageView,
}

impl DepthBuffer {
    pub fn new(vulkan: &VulkanInfo, swapchain: &Swapchain) -> Self {
        let mut format_option = None;
        for format in &[vk::Format::D32_SFLOAT, vk::Format::D32_SFLOAT_S8_UINT] {
            let device_format_properties = unsafe {
                vulkan
                    .instance
                    .get_physical_device_format_properties(vulkan.physical_device, *format)
            };
            if device_format_properties
                .optimal_tiling_features
                .contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
            {
                format_option = Some(*format);
            }
        }

        let format = format_option.expect("Vulkan: No supported depth buffer formats.");

        let image_create_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D {
                width: swapchain.surface_extent.width,
                height: swapchain.surface_extent.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let image = allocator::allocate_image(
            vulkan,
            &image_create_info,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        let image_view_subresource_range = vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::DEPTH)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1);

        let image_view_create_info = vk::ImageViewCreateInfo::builder()
            .image(image.image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(*image_view_subresource_range);

        let image_view = unsafe {
            vulkan
                .device
                .create_image_view(&image_view_create_info, None)
                .expect("Vulkan: Unable to create depth buffer image view.")
        };

        DepthBuffer {
            format,
            image,
            image_view,
        }
    }

    pub unsafe fn destroy(&self, vulkan: &VulkanInfo) {
        vulkan.device.destroy_image_view(self.image_view, None);
        allocator::free_image_unsafe(vulkan, &self.image);
    }
}
