use crate::renderer::bindless_texture::record_qtree::RecordQTree;

pub struct BindlessTexture {
    format: wgpu::TextureFormat,
    device: wgpu::Device,
    layers: u32,
    queue: wgpu::Queue,
    tex: wgpu::Texture,
    owned_blocks: Vec<Option<RecordQTree>>,
    has_updated: bool,
    max_depth: u32,
}

pub struct BindlessTextureHandle {
    offset: [i32; 3], // x, y, z
    limits: [i32; 2], // uv limit, if tex smaller than block
}

impl BindlessTexture {
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        format: wgpu::TextureFormat,
        layers: u32,
    ) -> Self {
        let max_dims = device.limits().max_texture_dimension_2d;
        let max_depth = max_dims.ilog2();

        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("master texture bindless"),
            size: wgpu::Extent3d {
                width: max_dims,
                height: max_dims,
                depth_or_array_layers: layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        return Self {
            tex,
            device,
            format,
            layers,
            queue,
            max_depth,
            owned_blocks: vec![],
            has_updated: false,
        };
    }

    fn get_image_handle(self: Self, image_max_dim: u32) -> Option<BindlessTextureHandle> {
        // ex: say we have a capacity of 12 (8k image)
        // and we get a image of 2 (4px image) to write.
        // we'll need to go to a depth level of 12 - 2 = 10 from top
        let depth_level_request = self.max_depth - ((image_max_dim - 1).ilog2() + 1); // (n - 1).ilog2() + 1 ceils the ilog2

        // check availability
        let u = 0; // log u
        let v = 0; // log v
        let mut i = 0; // layer

        while i < self.layers {
            let block = &self.owned_blocks[i as usize];

            match block {
                Some(blck) => {
                    let s = 3;
                    BindlessTextureHandle {
                        offset: [1, 2, 3],
                        limits: [1, 2],
                    };
                }
                None => {
                    let mut new_record = RecordQTree::new();
                    for j in 0..=depth_level_request {
                        new_record.add(1);

                        if let Some(val) = new_record.get(1) {
                            let p = *val;
                        } else {
                            new_record = RecordQTree::new();
                        };
                    }

                    return Some(BindlessTextureHandle {
                        offset: [0, 0, i as i32],
                        limits: [1, 2],
                    });
                }
            }

            i += 1;
        }

        return None;
    }

    // pub async fn create_texture_from_url(self: Self, url: &str) {}

    // make a new texture array twice the size and copy the old texture to it
    fn grow(mut self: Self) {
        let max_layer_count = self.device.limits().max_texture_array_layers;
        if self.layers == max_layer_count {
            return;
        }

        let max_dims = self.device.limits().max_texture_dimension_2d;
        let new_layers = std::cmp::min(max_layer_count, 2 * self.layers);

        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("master texture bindless"),
            size: wgpu::Extent3d {
                width: max_dims,
                height: max_dims,
                depth_or_array_layers: new_layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("master texture bindless grower"),
            });

        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.tex,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: max_dims,
                height: max_dims,
                depth_or_array_layers: self.layers,
            },
        );

        self.queue.submit(Some(encoder.finish()));

        self.layers = new_layers;
    }
}
