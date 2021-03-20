use super::VulkanInfo;
use ash::{extensions::khr, version::DeviceV1_0, vk};

pub struct Swapchain {
    pub surface_extent: vk::Extent2D,
    pub surface_format: vk::SurfaceFormatKHR,
    swapchain_loader: khr::Swapchain,
    swapchain: vk::SwapchainKHR,
    pub image_views: Vec<vk::ImageView>,
}

impl Swapchain {
    pub fn new(vulkan: &VulkanInfo) -> Self {
        // swapchain

        let surface_capabilities = unsafe {
            vulkan
                .surface_loader
                .get_physical_device_surface_capabilities(vulkan.physical_device, vulkan.surface)
                .expect("Vulkan: Unable to get physical device surface capabilities.")
        };

        let device_surface_formats = unsafe {
            vulkan
                .surface_loader
                .get_physical_device_surface_formats(vulkan.physical_device, vulkan.surface)
                .expect("Vulkan: Unable to get physical device surface formats.")
        };

        let surface_extent = surface_capabilities.current_extent;

        let mut surface_format = device_surface_formats
            .first()
            .expect("Vulkan: No valid swapchain surface formats.")
            .clone();

        // search for preferred image format
        for device_surface_format in &device_surface_formats {
            if device_surface_format.format == vk::Format::B8G8R8_UNORM
                && device_surface_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            {
                surface_format = device_surface_format.clone();
                break;
            }
        }

        let mut min_image_count = surface_capabilities.min_image_count + 1;
        if surface_capabilities.max_image_count != 0
            && min_image_count > surface_capabilities.max_image_count
        {
            min_image_count = surface_capabilities.max_image_count;
        }

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(vulkan.surface)
            .min_image_count(min_image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(surface_extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::FIFO)
            .clipped(true);

        let swapchain_loader = khr::Swapchain::new(&vulkan.instance, &vulkan.device);
        let swapchain = unsafe {
            swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .expect("Vulkan: Unable to create swapchain.")
        };

        assert_eq!(
            vulkan.device_queues.graphics_queue.family_index,
            vulkan.device_queues.present_queue.family_index,
            "Vulkan: Separate graphics and present queue families is unsupported."
        );

        // image views

        let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain).unwrap() };

        let mut image_views = Vec::with_capacity(swapchain_images.len());

        for swapchain_image in swapchain_images {
            let image_view_subresource_range = vk::ImageSubresourceRange::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1);

            let image_view_create_info = vk::ImageViewCreateInfo::builder()
                .image(swapchain_image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(surface_format.format)
                .subresource_range(*image_view_subresource_range);

            image_views.push(unsafe {
                vulkan
                    .device
                    .create_image_view(&image_view_create_info, None)
                    .expect("Vulkan: Unable to create swapchain image view.")
            });
        }

        Swapchain {
            surface_extent,
            surface_format,
            swapchain_loader,
            swapchain,
            image_views,
        }
    }

    pub unsafe fn destroy(&self, vulkan: &VulkanInfo) {
        for image_view in &self.image_views {
            vulkan.device.destroy_image_view(*image_view, None);
        }
        self.swapchain_loader
            .destroy_swapchain(self.swapchain, None);
    }

    pub fn acquire_next_image(&self, acquire_semaphore: vk::Semaphore) -> u32 {
        unsafe {
            self.swapchain_loader
                .acquire_next_image(
                    self.swapchain,
                    u64::MAX,
                    acquire_semaphore,
                    vk::Fence::null(),
                )
                .expect("Vulkan: Failed to acquire next swapchain image.")
                .0
        }
    }

    pub fn present(&self, vulkan: &VulkanInfo, wait_semaphore: vk::Semaphore, image_index: u32) {
        let wait_semaphores = [wait_semaphore];
        let swapchains = [self.swapchain];
        let image_indices = [image_index];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        unsafe {
            self.swapchain_loader
                .queue_present(vulkan.device_queues.present_queue.queue, &present_info)
                .unwrap();
        }
    }
}
