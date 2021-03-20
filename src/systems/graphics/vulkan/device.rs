use super::VulkanInfo;
use ash::{
    extensions::khr,
    version::{DeviceV1_0, InstanceV1_0},
    vk,
};
use std::ffi::CStr;

struct QueueFamiliesInfo {
    graphics_family_index: u32,
    present_family_index: u32,
    transfer_family_index: u32,
}

impl QueueFamiliesInfo {
    fn get_unique_family_indices(&self) -> Vec<u32> {
        let mut unique_family_indices = Vec::with_capacity(3);
        unique_family_indices.push(self.graphics_family_index);

        if !unique_family_indices.contains(&self.present_family_index) {
            unique_family_indices.push(self.present_family_index);
        }

        if !unique_family_indices.contains(&self.transfer_family_index) {
            unique_family_indices.push(self.transfer_family_index);
        }

        unique_family_indices
    }
}

pub struct Queue {
    pub queue: vk::Queue,
    pub family_index: u32,
}

pub struct Queues {
    pub graphics_queue: Queue,
    pub present_queue: Queue,
    pub transfer_queue: Queue,
}

pub fn new(
    instance: &ash::Instance,
    surface_loader: &khr::Surface,
    surface: vk::SurfaceKHR,
) -> (vk::PhysicalDevice, ash::Device, Queues) {
    let required_device_extensions = [khr::Swapchain::name()];
    let portability_subset_extension = std::ffi::CString::new("VK_KHR_portability_subset").unwrap();

    // select physical device

    let physical_device = unsafe {
        select_physical_device(
            instance,
            surface_loader,
            surface,
            &required_device_extensions,
        )
        .expect("Vulkan: No suitable physical device found.")
    };

    // create logical device

    let mut queue_create_infos = Vec::new();
    let device_queue_families_info =
        get_physical_device_queue_families_info(instance, surface_loader, surface, physical_device)
            .unwrap();
    let queue_priorities = vec![0.0];
    for queue_family_index in device_queue_families_info.get_unique_family_indices() {
        let queue_create_info = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .queue_priorities(&queue_priorities);
        queue_create_infos.push(queue_create_info.build());
    }

    let enabled_extension_names = select_physical_device_extensions(
        instance,
        physical_device,
        &required_device_extensions,
        portability_subset_extension.as_c_str(),
    );
    let enabled_extension_name_ptrs = enabled_extension_names
        .iter()
        .map(|extension| extension.as_ptr())
        .collect::<Vec<_>>();

    let device_create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_create_infos)
        .enabled_extension_names(&enabled_extension_name_ptrs);

    let device = unsafe {
        instance
            .create_device(physical_device, &device_create_info, None)
            .expect("Vulkan: Unable to create device.")
    };

    // retrieve queue handles

    let graphics_queue =
        unsafe { device.get_device_queue(device_queue_families_info.graphics_family_index, 0) };
    let present_queue =
        unsafe { device.get_device_queue(device_queue_families_info.present_family_index, 0) };
    let transfer_queue =
        unsafe { device.get_device_queue(device_queue_families_info.transfer_family_index, 0) };

    let device_queues = Queues {
        graphics_queue: Queue {
            queue: graphics_queue,
            family_index: device_queue_families_info.graphics_family_index,
        },
        present_queue: Queue {
            queue: present_queue,
            family_index: device_queue_families_info.present_family_index,
        },
        transfer_queue: Queue {
            queue: transfer_queue,
            family_index: device_queue_families_info.transfer_family_index,
        },
    };

    (physical_device, device, device_queues)
}

pub unsafe fn destroy(vulkan: &VulkanInfo) {
    vulkan.device.destroy_device(None);
}

unsafe fn select_physical_device(
    instance: &ash::Instance,
    surface_loader: &khr::Surface,
    surface: vk::SurfaceKHR,
    required_device_extensions: &[&CStr],
) -> Option<vk::PhysicalDevice> {
    let mut selected_physical_device = None;

    for physical_device in instance.enumerate_physical_devices().unwrap() {
        let device_extensions = instance
            .enumerate_device_extension_properties(physical_device)
            .unwrap();

        let mut matching_extensions = 0;
        for required_device_extension in required_device_extensions {
            for device_extension in &device_extensions {
                let device_extension_cstr =
                    CStr::from_ptr(device_extension.extension_name.as_ptr());
                if device_extension_cstr == *required_device_extension {
                    matching_extensions += 1;
                }
            }
        }
        if matching_extensions != required_device_extensions.len() {
            continue;
        }

        let surface_format_count = surface_loader
            .get_physical_device_surface_formats(physical_device, surface)
            .unwrap()
            .len();
        let surface_present_mode_count = surface_loader
            .get_physical_device_surface_present_modes(physical_device, surface)
            .unwrap()
            .len();
        if surface_format_count == 0 || surface_present_mode_count == 0 {
            continue;
        }

        if get_physical_device_queue_families_info(
            instance,
            surface_loader,
            surface,
            physical_device,
        )
        .is_none()
        {
            continue;
        }

        match selected_physical_device {
            None => {
                selected_physical_device = Some(physical_device);
            }
            Some(_) => {
                // prefer discrete device
                let device_properties = instance.get_physical_device_properties(physical_device);
                if device_properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
                    selected_physical_device = Some(physical_device);
                }
            }
        }
    }

    selected_physical_device
}

fn select_physical_device_extensions<'a>(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    required_device_extensions: &'a [&CStr],
    portability_subset_extension: &'a CStr,
) -> Vec<&'a CStr> {
    let mut seleced_device_extensions = required_device_extensions.to_vec();

    let device_extensions = unsafe {
        instance
            .enumerate_device_extension_properties(physical_device)
            .unwrap()
    };

    for device_extension in &device_extensions {
        let device_extension_cstr =
            unsafe { CStr::from_ptr(device_extension.extension_name.as_ptr()) };
        // if device supports the portability subset extension, it must be enabled
        if device_extension_cstr == portability_subset_extension {
            seleced_device_extensions.push(portability_subset_extension);
        }
    }

    seleced_device_extensions
}

fn get_physical_device_queue_families_info(
    instance: &ash::Instance,
    surface_loader: &khr::Surface,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
) -> Option<QueueFamiliesInfo> {
    let physical_device_queue_family_properties =
        unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

    let mut graphics_family_index = None;
    let mut present_family_index = None;
    let mut transfer_family_index = None;

    for (i, queue_family_properties) in physical_device_queue_family_properties.iter().enumerate() {
        let has_graphics_support = queue_family_properties
            .queue_flags
            .contains(vk::QueueFlags::GRAPHICS);
        let has_transfer_support = queue_family_properties
            .queue_flags
            .contains(vk::QueueFlags::TRANSFER);
        let has_compute_support = queue_family_properties
            .queue_flags
            .contains(vk::QueueFlags::COMPUTE);
        let has_present_support = unsafe {
            surface_loader
                .get_physical_device_surface_support(physical_device, i as u32, surface)
                .unwrap()
        };

        if has_graphics_support {
            if graphics_family_index == None {
                graphics_family_index = Some(i);
            }
        }

        if has_present_support {
            if has_graphics_support {
                // prefer graphics and present queue families to be the same
                graphics_family_index = Some(i);
                present_family_index = Some(i);
            } else if present_family_index == None {
                present_family_index = Some(i);
            }
        }

        if has_transfer_support {
            if transfer_family_index == None {
                transfer_family_index = Some(i);
            } else if graphics_family_index != Some(i) && !has_compute_support {
                // prefer dedicated transfer queue family
                transfer_family_index = Some(i);
            }
        }
    }

    if graphics_family_index.is_none()
        || present_family_index.is_none()
        || transfer_family_index.is_none()
    {
        None
    } else {
        Some(QueueFamiliesInfo {
            graphics_family_index: graphics_family_index.unwrap() as u32,
            present_family_index: present_family_index.unwrap() as u32,
            transfer_family_index: transfer_family_index.unwrap() as u32,
        })
    }
}
