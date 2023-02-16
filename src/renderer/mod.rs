pub mod debug;
pub mod swapchain;
pub mod pipeline;
pub mod surface;
pub mod command_pools;
pub mod device;

use ash::vk;
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
    fn used_layer_names() -> Vec<std::ffi::CString> {
        vec![
            std::ffi::CString::new("VK_LAYER_KHRONOS_validation").unwrap()
        ]
    }

    fn used_extensions() -> Vec<*const i8> {
        vec![
            ash::extensions::ext::DebugUtils::name().as_ptr(),
            ash::extensions::khr::Surface::name().as_ptr(),
            ash::extensions::khr::XlibSurface::name().as_ptr(),
        ]
    }

    pub fn new(
        window: winit::window::Window,
    ) -> Result<VulkanRenderer, Box<dyn std::error::Error>> {
        let entry = ash::Entry::linked();
        let used_layer_names = Self::used_layer_names();
        let used_layers = used_layer_names.iter()
            .map(|layer_name| layer_name.as_ptr())
            .collect();
        let used_extensions = Self::used_extensions();
        let instance = Self::create_instance(&entry, &used_layers, &used_extensions)?;
        let debug = Debug::new(&entry, &instance)?;
        let surfaces = Surface::new(&window, &entry, &instance)?;
        let device = Device::new(&instance, &used_layers)?;
        let mut swapchain = Swapchain::new(
            &instance, 
            &surfaces, 
            &device,
        )?;
        let renderpass = Self::create_renderpass(
            &device.logical_device, 
            swapchain.surface_format.format
        )?;
        swapchain.create_framebuffer(&device.logical_device, renderpass)?;
        let pipeline = Pipeline::new(
            &instance,
            &device.physical_device,
            &device.logical_device, 
            &swapchain, 
            &renderpass,
        )?;
        let command_pools = CommandPools::new(&device.logical_device, &device.queue_families)?;
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

    fn create_instance(
        entry: &ash::Entry,
        layer_name_pointers: &Vec<*const i8>,
        extension_name_pointers: &Vec<*const i8>,
    ) -> Result<ash::Instance, vk::Result> {
        let enginename = std::ffi::CString::new("UnknownGameEngine").unwrap();
        let appname = std::ffi::CString::new("The Black Window").unwrap();
        let app_info = vk::ApplicationInfo::builder()
            .engine_name(&enginename)
            .application_name(&appname)
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_version(vk::make_api_version(0, 0, 1, 0))
            .api_version(vk::API_VERSION_1_1);
        let instance_create_info = vk::InstanceCreateInfo::builder() 
            .application_info(&app_info)
            .enabled_layer_names(&layer_name_pointers)
            .enabled_extension_names(&extension_name_pointers);
        unsafe { entry.create_instance(&instance_create_info, None) }
    }

    fn create_renderpass(
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

