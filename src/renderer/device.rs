use ash::{vk, version::{InstanceV1_0, DeviceV1_0}};

pub struct Queues {
    pub graphics_queue: vk::Queue,
    pub transfer_queue: vk::Queue,
}

pub struct QueueFamilies {
    pub graphics_q_index: Option<u32>,
    pub transfer_q_index: Option<u32>,
}

impl QueueFamilies {
    pub fn init(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
    ) -> Result<QueueFamilies, vk::Result> {
        let queuefamilyproperties = unsafe {
            instance.get_physical_device_queue_family_properties(physical_device)
        };
        let mut found_graphics_q_index = None;
        let mut found_transfer_q_index = None;
        for (index, qfam) in queuefamilyproperties.iter().enumerate() {
            if qfam.queue_count > 0 
                && qfam.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                found_graphics_q_index = Some(index as u32);
            }
            if qfam.queue_count > 0 && qfam.queue_flags.contains(vk::QueueFlags::TRANSFER) {
                if found_transfer_q_index.is_none() 
                    || !qfam.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                        found_transfer_q_index = Some(index as u32);
                    }
            }
        }
        Ok(QueueFamilies{
            graphics_q_index: found_graphics_q_index,
            transfer_q_index: found_transfer_q_index,
        })
    }
}


pub struct Device {
    pub physical_device: vk::PhysicalDevice,
    pub logical_device: ash::Device,
    pub queue_families: QueueFamilies,
    pub queues: Queues,
}

impl Device {
    pub fn init(
        instance: &ash::Instance,
        layer_names: &[&str],
    ) -> Result<Device, vk::Result> {
        let physical_device = Self::get_physical_device(instance)?;
        let queue_families = QueueFamilies::init(instance, physical_device)?;
        let layer_names_c: Vec<std::ffi::CString> = layer_names
            .iter()
            .map(|&layer_name| std::ffi::CString::new(layer_name).unwrap())
            .collect();
        let layer_name_pointers: Vec<*const i8> = layer_names_c
            .iter()
            .map(|layer_name| layer_name.as_ptr())
            .collect();
        let priorities = [1.0f32];
        let queue_infos = [
            vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(queue_families.graphics_q_index.unwrap())
                .queue_priorities(&priorities)
                .build(),
            // TODO: Transfer and graphics queue are the same, so are the indices (0, 0), throws error
            // vk::DeviceQueueCreateInfo::builder()
            //     .queue_family_index(qfamindices.1)
            //     .queue_priorities(&priorities)
            //     .build(),
        ];

        let device_extension_name_pointers: Vec<*const i8> =
            vec![ash::extensions::khr::Swapchain::name().as_ptr()];
        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_infos)
            .enabled_extension_names(&device_extension_name_pointers)
            .enabled_layer_names(&layer_name_pointers);
        let logical_device = 
            unsafe { instance.create_device(physical_device, &device_create_info, None)? };
        let graphics_queue = 
            unsafe { logical_device.get_device_queue(queue_families.graphics_q_index.unwrap(), 0) };
        let transfer_queue = 
            unsafe { logical_device.get_device_queue(queue_families.transfer_q_index.unwrap(), 0) };

        Ok(Device {
            physical_device,
            logical_device,
            queue_families,
            queues: Queues {
                transfer_queue,
                graphics_queue,
            }
        })
    }

    fn get_physical_device(
        instance: &ash::Instance
    ) -> Result<vk::PhysicalDevice, vk::Result> {
        let phys_devs = unsafe { instance.enumerate_physical_devices()? };
        let mut chosen = None;
        for p in phys_devs {
            let properties = unsafe { instance.get_physical_device_properties(p) };
            chosen = Some(p);
            break;
        }
        Ok(chosen.unwrap())
    }

    pub unsafe fn cleanup(&self) {
        self.logical_device.destroy_device(None);
    }
}

