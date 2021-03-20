use super::depth_buffer::DepthBuffer;
use super::swapchain::Swapchain;
use super::VulkanInfo;
use ash::{version::DeviceV1_0, vk};

pub fn new(
    vulkan: &VulkanInfo,
    swapchain: &Swapchain,
    depth_buffer: &DepthBuffer,
) -> vk::RenderPass {
    // attachments

    let color_attachment_description = vk::AttachmentDescription::builder()
        .format(swapchain.surface_format.format)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

    let depth_attachment_description = vk::AttachmentDescription::builder()
        .format(depth_buffer.format)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::DONT_CARE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

    let attachments = [
        color_attachment_description.build(),
        depth_attachment_description.build(),
    ];

    // subpass

    let color_attachment_reference = vk::AttachmentReference::builder()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let depth_attachment_reference = vk::AttachmentReference::builder()
        .attachment(1)
        .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

    let color_attachments = [color_attachment_reference.build()];

    let subpass_description = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_attachments)
        .depth_stencil_attachment(&depth_attachment_reference);

    let subpasses = [subpass_description.build()];

    // render pass

    let render_pass_create_info = vk::RenderPassCreateInfo::builder()
        .attachments(&attachments)
        .subpasses(&subpasses);

    unsafe {
        vulkan
            .device
            .create_render_pass(&render_pass_create_info, None)
            .expect("Vulkan: Unable to create rener pass.")
    }
}

pub unsafe fn destroy(render_pass: vk::RenderPass, vulkan: &VulkanInfo) {
    vulkan.device.destroy_render_pass(render_pass, None);
}
