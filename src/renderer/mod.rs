pub mod debug;
pub mod swapchain;
pub mod pipeline;
pub mod surface;
pub mod command_pools;
pub mod device;

use ash::{vk, version::{InstanceV1_0, DeviceV1_0, EntryV1_0}};
use debug::Debug;
use swapchain::Swapchain;
use pipeline::Pipeline;
use surface::Surface;
use command_pools::CommandPools;
use device::Device;

pub struct VulkanRenderer {
    pub window: winit::window::Window,
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub debug: std::mem::ManuallyDrop<Debug>,
    pub surfaces: std::mem::ManuallyDrop<Surface>,
    pub device: Device,
    pub swapchain: Swapchain,
    pub renderpass: vk::RenderPass,
    pub pipeline: Pipeline,
    pub pools: CommandPools,
    pub commandbuffers: Vec<vk::CommandBuffer>,
}

impl VulkanRenderer {
    pub fn init(
        window: winit::window::Window,
    ) -> Result<VulkanRenderer, Box<dyn std::error::Error>> {
        let entry = ash::Entry::new()?;
        let layer_names = vec!["VK_LAYER_KHRONOS_validation"];
        let instance = Self::init_instance(&entry, &layer_names)?;
        let debug = Debug::init(&entry, &instance)?;
        let surfaces = Surface::init(&window, &entry, &instance)?;
        let device = Device::init(&instance, &layer_names)?;
        let mut swapchain = Swapchain::init(
            &instance, 
            &surfaces, 
            &device,
        )?;
        let renderpass = Self::init_renderpass(
            &device.logical_device, 
            swapchain.surface_format.format
        )?;
        swapchain.create_framebuffer(&device.logical_device, renderpass)?;
        let pipeline = Pipeline::init(&device.logical_device, &swapchain, &renderpass)?;
        let command_pools = CommandPools::init(&device.logical_device, &device.queue_families)?;
        let commandbuffers =
            CommandPools::create_commandbuffers(&device.logical_device, &command_pools, swapchain.framebuffers.len())?;
        Self::fill_commandbuffers(
            &commandbuffers,
            &device.logical_device,
            &renderpass,
            &swapchain, 
            &pipeline,
        )?;
        Ok(VulkanRenderer { 
            window,
            entry, 
            instance, 
            debug: std::mem::ManuallyDrop::new(debug), 
            surfaces: std::mem::ManuallyDrop::new(surfaces), 
            device,
            swapchain,
            renderpass,
            pipeline,
            pools: command_pools,
            commandbuffers,
        })
    }

    fn init_instance(
        entry: &ash::Entry,
        layer_names: &[&str]
    ) -> Result<ash::Instance, ash::InstanceError> {
        let enginename = std::ffi::CString::new("UnknownGameEngine").unwrap();
        let appname = std::ffi::CString::new("The Black Window").unwrap();
        let app_info = vk::ApplicationInfo::builder()
            .application_name(&appname)
            .application_version(vk::make_version(0, 0, 1))
            .engine_name(&enginename)
            .engine_version(vk::make_version(0, 42, 0))
            .api_version(vk::make_version(1, 0, 106));
        let layer_names_c: Vec<std::ffi::CString> = layer_names
            .iter()
            .map(|&layer_name| std::ffi::CString::new(layer_name).unwrap())
            .collect();
        let layer_name_pointers: Vec<*const i8> = layer_names_c
            .iter()
            .map(|layer_name| layer_name.as_ptr())
            .collect();
        let extension_name_pointers: Vec<*const i8> = vec![
            ash::extensions::ext::DebugUtils::name().as_ptr(),
            ash::extensions::khr::Surface::name().as_ptr(),
            ash::extensions::khr::XlibSurface::name().as_ptr(),
        ];

        // Create instance and instance info
        let instance_create_info = vk::InstanceCreateInfo::builder() 
            .application_info(&app_info)
            .enabled_layer_names(&layer_name_pointers)
            .enabled_extension_names(&extension_name_pointers);
        unsafe { entry.create_instance(&instance_create_info, None) }
    }

    fn init_renderpass(
        logical_device: &ash::Device,
        format: vk::Format,
    ) -> Result<vk::RenderPass, vk::Result> {
        let attachments = [vk::AttachmentDescription::builder()
            .format(format)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .samples(vk::SampleCountFlags::TYPE_1)
            .build()];
        let color_attachment_references = [vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }];
        let subpasses = [vk::SubpassDescription::builder()
            .color_attachments(&color_attachment_references)
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS).build()];
        let subpass_dependencies = [vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_subpass(0)
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(
                vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            )
            .build()];
        let renderpass_info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&subpass_dependencies);
        let renderpass = 
            unsafe { logical_device.create_render_pass(&renderpass_info, None)? };
        Ok(renderpass)
    }


    fn fill_commandbuffers(
        commandbuffers: &[vk::CommandBuffer],
        logical_device: &ash::Device,
        renderpass: &vk::RenderPass,
        swapchain: &Swapchain,
        pipeline: &Pipeline,
    ) -> Result<(), vk::Result> {
        for (i, &commandbuffer) in commandbuffers.iter().enumerate() {
            let commmandbuffer_begininfo = vk::CommandBufferBeginInfo::builder();
            unsafe {
                logical_device.begin_command_buffer(commandbuffer, &commmandbuffer_begininfo)?;
            }
            let clearvalues = [vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.08, 1.0],
                },
            }];
            let renderpass_begininfo = vk::RenderPassBeginInfo::builder()
                .render_pass(*renderpass)
                .framebuffer(swapchain.framebuffers[i])
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: swapchain.extent,
                })
                .clear_values(&clearvalues);
                unsafe {
                    logical_device.cmd_begin_render_pass(
                        commandbuffer, 
                        &renderpass_begininfo, 
                        vk::SubpassContents::INLINE,
                    );
                    logical_device.cmd_bind_pipeline(
                        commandbuffer, 
                        vk::PipelineBindPoint::GRAPHICS, 
                        pipeline.pipeline
                    );
                    logical_device.cmd_draw(commandbuffer, 1, 1, 0, 0);
                    logical_device.cmd_end_render_pass(commandbuffer);
                    logical_device.end_command_buffer(commandbuffer)?;
                }
        }
        Ok(())
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
         unsafe { 
             self.device
                 .logical_device
                 .device_wait_idle()
                 .expect("something wrong while wating");
             self.pools.cleanup(&self.device.logical_device);
             self.pipeline.cleanup(&self.device.logical_device);
             self.device.logical_device.destroy_render_pass(self.renderpass, None);
             self.swapchain.cleanup(&self.device.logical_device);
             self.device.logical_device.destroy_device(None);
             std::mem::ManuallyDrop::drop(&mut self.surfaces);
             self.device.cleanup();
             std::mem::ManuallyDrop::drop(&mut self.debug);
             self.instance.destroy_instance(None)
         };       
    }
}

