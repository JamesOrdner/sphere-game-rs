use super::VulkanInfo;
use ash::{
    extensions::{ext::DebugUtils, khr},
    version::{EntryV1_0, InstanceV1_0},
    vk, Entry,
};
use std::ffi::{CStr, CString};
use winit::window::Window;

pub fn new(window: &Window) -> (ash::Instance, khr::Surface, vk::SurfaceKHR) {
    let entry = Entry::new().unwrap();

    let application_info = vk::ApplicationInfo::builder()
        .application_version(vk::make_version(0, 1, 0))
        .api_version(vk::make_version(1, 1, 0));

    #[cfg(debug_assertions)]
    let layer_names = [CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
    #[cfg(not(debug_assertions))]
    let layer_names: [CString; 0] = [];
    let layer_name_ptrs: Vec<*const i8> = layer_names
        .iter()
        .map(|raw_name| raw_name.as_ptr())
        .collect();

    let extension_names = required_extensions(window);
    let mut extension_name_ptrs = extension_names
        .iter()
        .map(|extension| extension.as_ptr())
        .collect::<Vec<_>>();
    extension_name_ptrs.push(DebugUtils::name().as_ptr());

    let instance_create_info = vk::InstanceCreateInfo::builder()
        .application_info(&application_info)
        .enabled_layer_names(&layer_name_ptrs)
        .enabled_extension_names(&extension_name_ptrs);

    let instance = unsafe {
        entry
            .create_instance(&instance_create_info, None)
            .expect("Vulkan: Could not create instance.")
    };

    let surface_loader = khr::Surface::new(&entry, &instance);

    let surface = unsafe {
        ash_window::create_surface(&entry, &instance, window, None)
            .expect("Vulkan: Could not create surface.")
    };

    (instance, surface_loader, surface)
}

pub unsafe fn destroy(vulkan: &VulkanInfo) {
    vulkan.surface_loader.destroy_surface(vulkan.surface, None);
    vulkan.instance.destroy_instance(None);
}

fn required_extensions(window: &Window) -> Vec<&CStr> {
    let mut extensions = ash_window::enumerate_required_extensions(window).unwrap();
    if cfg!(debug_assertions) {
        extensions.push(DebugUtils::name());
    }
    extensions
}
