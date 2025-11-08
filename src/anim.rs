use {
    color_eyre::eyre::Result,
    image::{
        AnimationDecoder, DynamicImage, Frame,
        codecs::{gif::GifDecoder, webp::WebPDecoder},
    },
    ratatui_image::{ResizeEncodeRender, picker::Picker, protocol::StatefulProtocol},
    std::{io::Cursor, time::Instant},
};

impl ResizeEncodeRender for ImageProtocol {
    fn render(&mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        match self {
            ImageProtocol::Single(p) => p.render(area, buf),
            ImageProtocol::Animated {
                frames, current, ..
            } => {
                frames[*current].render(area, buf);
            }
        }
    }

    fn resize_encode(&mut self, resize: &ratatui_image::Resize, area: ratatui::prelude::Rect) {
        match self {
            ImageProtocol::Single(p) => p.resize_encode(resize, area),
            ImageProtocol::Animated {
                frames, current, ..
            } => {
                frames[*current].resize_encode(resize, area);
            }
        }
    }

    fn needs_resize(
        &self,
        resize: &ratatui_image::Resize,
        area: ratatui::prelude::Rect,
    ) -> Option<ratatui::prelude::Rect> {
        match self {
            ImageProtocol::Single(p) => p.needs_resize(resize, area),
            ImageProtocol::Animated {
                frames, current, ..
            } => frames[*current].needs_resize(resize, area),
        }
    }
}

pub enum ImageProtocol {
    Single(StatefulProtocol),
    Animated {
        frames: Vec<StatefulProtocol>,
        delays_ms: Vec<u64>,
        current: usize,
        last_change: Instant,
    },
}

impl ImageProtocol {
    pub fn try_advance(&mut self) -> bool {
        match self {
            ImageProtocol::Animated {
                frames,
                delays_ms,
                current,
                last_change,
            } => {
                if frames.is_empty() || delays_ms.is_empty() {
                    return false;
                }
                let now = Instant::now();
                let delay = delays_ms.get(*current).copied().unwrap_or(100);
                if now.duration_since(*last_change).as_millis() >= delay as u128 {
                    *current = (*current + 1) % frames.len();
                    *last_change = now;
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn current_protocol_mut(&mut self) -> &mut StatefulProtocol {
        match self {
            ImageProtocol::Single(p) => p,
            ImageProtocol::Animated {
                frames, current, ..
            } => &mut frames[*current],
        }
    }
}

pub fn protocols_from_animated_bytes(bytes: &[u8], picker: &mut Picker) -> Result<ImageProtocol> {
    let cursor = Cursor::new(bytes);

    let frames_result: Result<Vec<Frame>, image::ImageError> =
        if let Ok(decoder) = GifDecoder::new(cursor.clone()) {
            decoder.into_frames().collect_frames()
        } else if let Ok(decoder) = WebPDecoder::new(cursor) {
            decoder.into_frames().collect_frames()
        } else {
            let img = image::load_from_memory(bytes)?;
            let proto = picker.new_resize_protocol(img);
            return Ok(ImageProtocol::Single(proto));
        };

    let frames = frames_result?;

    if frames.len() <= 1 {
        let img = image::load_from_memory(bytes)?;
        let proto = picker.new_resize_protocol(img);
        return Ok(ImageProtocol::Single(proto));
    }

    let mut protocols = Vec::with_capacity(frames.len());
    let mut delays_ms = Vec::with_capacity(frames.len());

    for frame in frames {
        let buf = frame.clone().into_buffer();
        let (delay_num, delay_den) = frame.delay().numer_denom_ms();
        let dyn_img = DynamicImage::ImageRgba8(buf);
        let mut proto = picker.new_resize_protocol(dyn_img);

        proto.resize_encode(
            &ratatui_image::Resize::Fit(None),
            ratatui::prelude::Rect::default(),
        );

        protocols.push(proto);

        let delay = if delay_num == 0 {
            100
        } else {
            (delay_num as f64 / delay_den as f64).round() as u64
        };
        delays_ms.push(delay);
    }

    Ok(ImageProtocol::Animated {
        frames: protocols,
        delays_ms,
        current: 0,
        last_change: Instant::now(),
    })
}
