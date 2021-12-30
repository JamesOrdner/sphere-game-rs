use super::depth_buffer::DepthBuffer;
use super::swapchain::Swapchain;
use super::VulkanInfo;
use ash::{version::DeviceV1_0, vk};

pub fn new(
    vulkan: &VulkanInfo,
    depth_buffer: &DepthBuffer,
    swapchain: &Swapchain,
    render_pass: vk::RenderPass,
) -> Vec<vk::Framebuffer> {
    let mut framebuffers = Vec::with_capacity(swapchain.image_views.len());

    for image_view in &swapchain.image_views {
        let attachments = [*image_view, depth_buffer.image_view];

        let framebuffer_create_info = vk::FramebufferCreateInfo::builder()
            .render_pass(render_pass)
            .attachments(&attachments)
            .width(swapchain.surface_extent.width)
            .height(swapchain.surface_extent.height)
            .layers(1);

        let framebuffer = unsafe {
            vulkan
                .device
                .create_framebuffer(&framebuffer_create_info, None)
                .expect("Vulkan: Unable to create framebuffer.")
        };

        framebuffers.push(framebuffer);
    }

    framebuffers
}

pub unsafe fn destroy(framebuffers: &[vk::Framebuffer], vulkan: &VulkanInfo) {
    for framebuffer in framebuffers {
        vulkan.device.destroy_framebuffer(*framebuffer, None);
    }
}
