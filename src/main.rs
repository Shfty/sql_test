mod loader;
use async_std::{stream::StreamExt, task};
use loader::*;
use wgpu::SwapChainDescriptor;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use std::{error::Error, ops::Deref, sync::Arc, time::Duration};

use sqlx::{sqlite::SqlitePoolOptions, Executor, Sqlite};

fn in_memory_database_uri(name: &str, shared: bool) -> String {
    format!(
        "file:{}?mode=memory{}",
        name,
        if shared { "&cache=shared" } else { "" }
    )
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize in-memory database
    let mem_uri = in_memory_database_uri("db", true);

    let pool = SqlitePoolOptions::new()
        .test_before_acquire(false)
        .min_connections(4)
        .max_connections(16)
        .idle_timeout(None)
        .max_lifetime(None)
        .connect(&mem_uri)
        .await?;

    load_db_to_memory(&dotenv::var("DATABASE_URL")?, &mem_uri).await?;

    let pool = Arc::new(pool);
    let game_loop_pool = pool.clone();
    task::spawn(async {
        let pool = game_loop_pool;
        loop {
            position_integrator(pool.deref()).await;
            ball_collision(pool.deref()).await;
            velocity_position_debugger(pool.deref()).await;
            task::sleep(Duration::from_secs_f64(1.0)).await;
        }
    });

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_visible(false)
        .build(&event_loop)
        .unwrap();
    let window_size = window.inner_size();
    println!("Window size: {:?}", window_size);

    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
    let surface = unsafe { instance.create_surface(&window) };

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
        })
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await?;

    let swap_chain = device.create_swap_chain(
        &surface,
        &SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: adapter.get_swap_chain_preferred_format(&surface),
            width: window_size.width,
            height: window_size.height,
            present_mode: wgpu::PresentMode::Fifo,
        },
    );

    let mut first_frame = true;
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::MainEventsCleared => {
                window.request_redraw();

                if first_frame {
                    window.set_visible(true);
                    first_frame = false;
                }
            }
            Event::RedrawRequested(_) => {
                let frame = swap_chain.get_current_frame().unwrap().output;
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                {
                    let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: &frame.view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.1,
                                    g: 0.2,
                                    b: 0.3,
                                    a: 1.0,
                                }),
                                store: true,
                            },
                        }],
                        depth_stencil_attachment: None,
                    });
                }

                // submit will accept anything that implements IntoIter
                queue.submit(std::iter::once(encoder.finish()));
            }
            _ => (),
        }
    });
}

async fn position_integrator<'a, E>(executor: E)
where
    E: Executor<'a, Database = Sqlite>,
{
    sqlx::query_file!("sql/position_integrator.sql")
        .execute(executor)
        .await
        .unwrap();
}

async fn ball_collision<'a, E>(executor: E)
where
    E: Executor<'a, Database = Sqlite>,
{
    sqlx::query_file!("sql/ball_collision.sql", -100, 100, -50, 50)
        .execute(executor)
        .await
        .unwrap();
}

async fn velocity_position_debugger<'a, E>(executor: E)
where
    E: Executor<'a, Database = Sqlite>,
{
    let mut results = sqlx::query!("SELECT * FROM view_velocity_position").fetch(executor);
    while let Some(result) = results.next().await {
        let result = result.unwrap();
        println!(
            "id: {:?}, vx: {:?}, vy: {:?}, px: {:?}, py: {:?}",
            result.id, result.vx, result.vy, result.px, result.py
        );
    }
}
