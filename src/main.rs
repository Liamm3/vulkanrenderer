mod renderer;

use ash::{version::DeviceV1_0, vk};
use renderer::VulkanRenderer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let eventloop = winit::event_loop::EventLoop::new();
    let window = winit::window::Window::new(&eventloop)?;
    let mut renderer = VulkanRenderer::init(window)?;

    use winit::event::{Event, WindowEvent};
    eventloop.run(move |event, _, controlflow| match event {
        Event::WindowEvent { 
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *controlflow = winit::event_loop::ControlFlow::Exit;
        },
        Event::MainEventsCleared => {
            // doing the work here
            renderer.window.request_redraw();
        },
        Event::RedrawRequested(_) => {
            // render here
            let (image_index, _) = unsafe {
                renderer
                    .swapchain
                    .swapchain_loader
                    .acquire_next_image(
                        renderer.swapchain.swapchain, 
                        std::u64::MAX,
                        renderer.swapchain.image_available[renderer.swapchain.current_image],
                        vk::Fence::null()
                    )
                    .expect("image aquisition trouble")
            };
            unsafe {
                renderer
                    .device
                    .logical_device
                    .wait_for_fences(
                        &[renderer.swapchain.may_begin_drawing[renderer.swapchain.current_image]],
                        true, 
                        std::u64::MAX
                    )
                    .expect("fence wainting");

                renderer
                    .device
                    .logical_device
                    .reset_fences(&[
                        renderer.swapchain.may_begin_drawing[renderer.swapchain.current_image]
                    ])
                    .expect("resetting fences");
            };
            let semaphores_available = 
                [renderer.swapchain.image_available[renderer.swapchain.current_image]];
            let waiting_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let semaphores_finished = 
                [renderer.swapchain.rendering_finished[renderer.swapchain.current_image]];
            let commandbuffers = [renderer.commandbuffers[image_index as usize]];
            let submit_info = [vk::SubmitInfo::builder()
                .wait_semaphores(&semaphores_available)
                .wait_dst_stage_mask(&waiting_stages)
                .command_buffers(&commandbuffers)
                .signal_semaphores(&semaphores_finished)
                .build()];
            unsafe {
                renderer
                    .device
                    .logical_device
                    .queue_submit(
                        renderer.device.queues.graphics_queue,
                        &submit_info, 
                        renderer.swapchain.may_begin_drawing[renderer.swapchain.current_image]
                    )
                    .expect("queue submission");
            };
            let swapchains = [renderer.swapchain.swapchain];
            let indices = [image_index];
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(&semaphores_finished)
                .swapchains(&swapchains)
                .image_indices(&indices);
            unsafe {
                renderer
                    .swapchain
                    .swapchain_loader
                    .queue_present(renderer.device.queues.graphics_queue, &present_info)
                    .expect("queue presentation");
            }
            renderer.swapchain.current_image =
                (renderer.swapchain.current_image + 1) % renderer.swapchain.amount_of_images as usize;
        },
        _ => {}
    });
}
