use crate::renderer::sprite::{build_quad, uv_rect, SpriteVertex};
use crate::renderer::texture::GpuTexture;
use bytemuck::cast_slice;
use glam::Mat4;
use std::collections::HashMap;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::*;

const QUAD_INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];
const MAX_SPRITES: usize = 10_000;

pub struct SpritePipeline {
    pipeline: RenderPipeline,
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
    texture_bind_group_layout: BindGroupLayout,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    texture_cache: HashMap<u64, (GpuTexture, BindGroup)>,
}

impl SpritePipeline {
    pub fn new(device: &Device, surface_format: TextureFormat) -> Self {
        let shader = device.create_shader_module(include_wgsl!("../shaders/sprite.wgsl"));

        let camera_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("camera"),
            size: std::mem::size_of::<[[f32; 4]; 4]>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("camera_bgl"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("camera_bg"),
            layout: &camera_bgl,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let texture_bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("texture_bgl"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("sprite_layout"),
            bind_group_layouts: &[&camera_bgl, &texture_bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("sprite_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[SpriteVertex::LAYOUT],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });

        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("sprite_vbo"),
            size: (MAX_SPRITES * 4 * std::mem::size_of::<SpriteVertex>()) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let indices: Vec<u16> = (0..MAX_SPRITES as u16)
            .flat_map(|i| QUAD_INDICES.map(|idx| idx + i * 4))
            .collect();
        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("sprite_ibo"),
            contents: cast_slice(&indices),
            usage: BufferUsages::INDEX,
        });

        Self {
            pipeline,
            camera_buffer,
            camera_bind_group,
            texture_bind_group_layout: texture_bgl,
            vertex_buffer,
            index_buffer,
            texture_cache: HashMap::new(),
        }
    }

    pub fn ensure_uploaded(
        &mut self,
        device: &Device,
        queue: &Queue,
        handle_id: u64,
        bytes: &[u8],
        label: &str,
    ) {
        if self.texture_cache.contains_key(&handle_id) {
            return;
        }
        let Ok(gpu_tex) = GpuTexture::from_bytes(device, queue, bytes, label) else {
            return;
        };
        let bg = device.create_bind_group(&BindGroupDescriptor {
            label: Some(label),
            layout: &self.texture_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&gpu_tex.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&gpu_tex.sampler),
                },
            ],
        });
        self.texture_cache.insert(handle_id, (gpu_tex, bg));
    }

    pub fn update_camera(&self, queue: &Queue, width: f32, height: f32) {
        let proj = Mat4::orthographic_rh(0.0, width, height, 0.0, -1.0, 1.0);
        queue.write_buffer(&self.camera_buffer, 0, cast_slice(&proj.to_cols_array_2d()));
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        device: &Device,
        queue: &Queue,
        view: &TextureView,
        encoder: &mut CommandEncoder,
        world: &hecs::World,
        assets: &crate::assets::AssetServer,
        texture_bytes: &HashMap<u64, Vec<u8>>,
    ) {
        use crate::ecs::components::{Sprite, Transform};

        let mut batches: HashMap<u64, Vec<[SpriteVertex; 4]>> = HashMap::new();
        for (_, (sprite, transform)) in world.query::<(&Sprite, &Transform)>().iter() {
            let sheet = match assets.get_sheet(&sprite.sheet) {
                Some(s) => s,
                None => continue,
            };
            let handle_id = sprite.sheet.id();
            let uv = uv_rect(sprite.frame, sheet.columns, sheet.rows);
            let size = [
                sheet.frame_width as f32 * transform.scale.x,
                sheet.frame_height as f32 * transform.scale.y,
            ];
            let quad = build_quad(
                [transform.position.x, transform.position.y],
                size,
                uv,
                sprite.color,
                sprite.flip_x,
                sprite.flip_y,
            );
            batches.entry(handle_id).or_default().push(quad);
        }

        for handle_id in batches.keys() {
            if let Some(bytes) = texture_bytes.get(handle_id) {
                self.ensure_uploaded(device, queue, *handle_id, bytes, "sprite");
            }
        }

        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("sprite_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

        let mut vertex_offset: u64 = 0;
        for (handle_id, quads) in &batches {
            let Some((_, bg)) = self.texture_cache.get(handle_id) else {
                continue;
            };

            let verts: Vec<SpriteVertex> = quads.iter().flat_map(|q| q.iter().copied()).collect();
            let byte_size = (verts.len() * std::mem::size_of::<SpriteVertex>()) as u64;

            if vertex_offset + byte_size
                > (MAX_SPRITES * 4 * std::mem::size_of::<SpriteVertex>()) as u64
            {
                break;
            }

            queue.write_buffer(&self.vertex_buffer, vertex_offset, cast_slice(&verts));

            render_pass.set_bind_group(1, bg, &[]);
            render_pass.set_vertex_buffer(
                0,
                self.vertex_buffer
                    .slice(vertex_offset..vertex_offset + byte_size),
            );
            render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
            render_pass.draw_indexed(0..(quads.len() * 6) as u32, 0, 0..1);

            vertex_offset += byte_size;
        }
    }
}
