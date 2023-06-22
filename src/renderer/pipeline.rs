use ash::vk;
use gpu_allocator::{vulkan::{Allocator, AllocatorCreateDesc, AllocationCreateDesc}, MemoryLocation};
use crate::renderer::swapchain::Swapchain;

pub struct Pipeline {
    pub pipeline: vk::Pipeline,
    layout: vk::PipelineLayout,
    allocator: Allocator,
}
impl Pipeline {
    pub fn new(
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
        logical_device: &ash::Device,
        swapchain: &Swapchain,
        renderpass: &vk::RenderPass,
    ) -> Result<Pipeline, vk::Result> {
        let vertexshader_createinfo = vk::ShaderModuleCreateInfo::builder()
            .code(
                vk_shader_macros::include_glsl!("./shaders/shader.vert", kind: vert),
            );
        let vertexshader_module =
            unsafe { logical_device.create_shader_module(&vertexshader_createinfo, None)? };
        let fragmentshader_createinfo = vk::ShaderModuleCreateInfo::builder()
            .code(vk_shader_macros::include_glsl!("./shaders/shader.frag"));
        let fragmentshader_module =
            unsafe { logical_device.create_shader_module(&fragmentshader_createinfo, None)? };
        let mainfunctionname = std::ffi::CString::new("main").unwrap();
        let vertexshader_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vertexshader_module)
            .name(&mainfunctionname);
        let fragmentshader_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(fragmentshader_module)
            .name(&mainfunctionname);
        let shader_stages = vec![vertexshader_stage.build(), fragmentshader_stage.build()];
        let vertex_attribute_descriptions = [vk::VertexInputAttributeDescription {
            binding: 0,
            location: 0,
            offset: 0,
            format: vk::Format::R32G32B32A32_SFLOAT,
        }];
        let vertex_binding_descriptions = [vk::VertexInputBindingDescription {
            binding: 0,
            stride: 16,
            input_rate: vk::VertexInputRate::VERTEX,
        }];
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_attribute_descriptions(&vertex_attribute_descriptions)
            .vertex_binding_descriptions(&vertex_binding_descriptions);
        let mut allocator = Allocator::new(&AllocatorCreateDesc {
            instance: instance.clone(),
            device: logical_device.clone(),
            physical_device: physical_device.clone(),
            debug_settings: Default::default(),
            buffer_device_address: false,
        }).unwrap();
        let vk_info = vk::BufferCreateInfo::builder()
            .size(16)
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER);
        let buffer = unsafe { logical_device.create_buffer(&vk_info, None) }?;
        let requirements = unsafe { logical_device.get_buffer_memory_requirements(buffer) };
        let allocation = allocator
            .allocate(&AllocationCreateDesc { name: "Example allocation",
                requirements,
                location: MemoryLocation::CpuToGpu,
                linear: true, // Buffers are always linear
            }).unwrap();
        unsafe { 
            logical_device.bind_buffer_memory(buffer, allocation.memory(), allocation.offset()).unwrap() 
        };
        // allocator.
        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::POINT_LIST);
        let viewports = [vk::Viewport {
            x: 0.,
            y: 0.,
            width: swapchain.extent.width as f32,
            height: swapchain.extent.height as f32,
            min_depth: 0.,
            max_depth: 1.,
        }];
        let scissors = [vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: swapchain.extent,
        }];
        let viewport_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors);
        let rasterizer_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .line_width(1.0)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .cull_mode(vk::CullModeFlags::NONE)
            .polygon_mode(vk::PolygonMode::FILL);
        let multisampler_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);
        let colorblend_attachments = [vk::PipelineColorBlendAttachmentState::builder()
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .alpha_blend_op(vk::BlendOp::ADD)
            .color_write_mask(
                vk::ColorComponentFlags::R
                | vk::ColorComponentFlags::G
                | vk::ColorComponentFlags::B
                | vk::ColorComponentFlags::A,
            )
            .build()];
        let colorblend_info = vk::PipelineColorBlendStateCreateInfo::builder()
            .attachments(&colorblend_attachments);
        let pipelinelayout_info = vk::PipelineLayoutCreateInfo::builder();
        let pipelinelayout = 
            unsafe { logical_device.create_pipeline_layout(&pipelinelayout_info, None) }?;
        let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly_info)
            .viewport_state(&viewport_info)
            .rasterization_state(&rasterizer_info)
            .multisample_state(&multisampler_info)
            .color_blend_state(&colorblend_info)
            .layout(pipelinelayout)
            .render_pass(*renderpass)
            .subpass(0);
        let graphicspipeline = unsafe {
            logical_device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[pipeline_info.build()],
                    None,
                )
                .expect("A problem with the pipeline creation")
        }[0];
        allocator.free(allocation).unwrap();
        unsafe {
            logical_device.destroy_buffer(buffer, None);
            logical_device.destroy_shader_module(fragmentshader_module, None);
            logical_device.destroy_shader_module(vertexshader_module, None);
        }
        Ok(Pipeline { 
            pipeline: graphicspipeline,
            layout: pipelinelayout,
            allocator,
        })
    }

    pub fn cleanup(&self, logical_device: &ash::Device) {
        unsafe {
            logical_device.destroy_pipeline(self.pipeline, None);
            logical_device.destroy_pipeline_layout(self.layout, None);
        }
    }
}

