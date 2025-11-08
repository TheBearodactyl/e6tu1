use {
    color_eyre::eyre::Result,
    image::{
        AnimationDecoder, DynamicImage, Frame,
        codecs::{gif::GifDecoder, webp::WebPDecoder},
    },
    ratatui_image::{ResizeEncodeRender, picker::Picker, protocol::StatefulProtocol},
    rayon::prelude::*,
    std::{io::Cursor, time::Instant},
};

pub enum ImageProtocol {
    Single(StatefulProtocol),
    Animated {
        frames: Vec<StatefulProtocol>,
        current: usize,
        last_change: Instant,
        frame_delay_ms: u64,
    },
}

impl ImageProtocol {
    pub fn try_advance(&mut self) -> bool {
        if let ImageProtocol::Animated {
            frames,
            current,
            last_change,
            frame_delay_ms,
        } = self
        {
            if frames.is_empty() {
                return false;
            }

            let now = Instant::now();
            let elapsed_ms = now.duration_since(*last_change).as_millis() as u64;

            if elapsed_ms >= *frame_delay_ms {
                let steps = (elapsed_ms / *frame_delay_ms) as usize;
                *current = (*current + steps) % frames.len();
                *last_change = now - std::time::Duration::from_millis(elapsed_ms % *frame_delay_ms);
                return true;
            }
        }
        false
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

impl ResizeEncodeRender for ImageProtocol {
    fn render(&mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        match self {
            ImageProtocol::Single(p) => p.render(area, buf),
            ImageProtocol::Animated {
                frames, current, ..
            } => frames[*current].render(area, buf),
        }
    }

    fn resize_encode(&mut self, resize: &ratatui_image::Resize, area: ratatui::prelude::Rect) {
        match self {
            ImageProtocol::Single(p) => p.resize_encode(resize, area),
            ImageProtocol::Animated { frames, .. } => {
                for frame in frames {
                    if frame.needs_resize(resize, area).is_some() {
                        frame.resize_encode(resize, area);
                    }
                }
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
            ImageProtocol::Animated { frames, .. } => {
                frames.first().and_then(|f| f.needs_resize(resize, area))
            }
        }
    }
}

pub fn protocols_from_animated_bytes(
    bytes: &[u8],
    picker: &mut Picker,
    fps: f64,
) -> Result<ImageProtocol> {
    let cursor = Cursor::new(bytes);

    let frames_result: Result<Vec<Frame>, image::ImageError> =
        if let Ok(decoder) = GifDecoder::new(cursor.clone()) {
            decoder.into_frames().collect_frames()
        } else if let Ok(decoder) = WebPDecoder::new(cursor) {
            decoder.into_frames().collect_frames()
        } else {
            let img = image::load_from_memory(bytes)?;
            return Ok(ImageProtocol::Single(picker.new_resize_protocol(img)));
        };

    let frames = frames_result?;
    if frames.len() <= 1 {
        let img = image::load_from_memory(bytes)?;
        return Ok(ImageProtocol::Single(picker.new_resize_protocol(img)));
    }

    let frame_delay_ms = (1000.0 / fps).round() as u64;

    let protocols: Vec<StatefulProtocol> = frames
        .into_par_iter()
        .map(|frame| {
            let buf = frame.into_buffer();
            let dyn_img = DynamicImage::ImageRgba8(buf);
            let mut proto = picker.new_resize_protocol(dyn_img);
            proto.resize_encode(
                &ratatui_image::Resize::Fit(None),
                ratatui::prelude::Rect::default(),
            );
            proto
        })
        .collect();

    Ok(ImageProtocol::Animated {
        frames: protocols,
        current: 0,
        last_change: Instant::now(),
        frame_delay_ms,
    })
}
