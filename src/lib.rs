use gif::{Encoder, Frame, Repeat};
use std::io::Cursor;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

/// Represents the dimensions of a GIF frame
///
/// This struct is used to pass width and height information between Rust and JavaScript.
/// The dimensions are stored as unsigned 16-bit integers to match GIF format specifications.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
pub struct Dimensions {
    /// Width of the GIF frame in pixels
    pub width: usize,
    /// Height of the GIF frame in pixels
    pub height: usize,
}

#[wasm_bindgen]
impl Dimensions {
    /// Creates a new Dimensions struct with the specified width and height
    ///
    /// # Arguments
    /// * `width` - The width of the frame in pixels
    /// * `height` - The height of the frame in pixels
    #[wasm_bindgen(constructor)]
    pub fn new(width: usize, height: usize) -> Dimensions {
        Dimensions { width, height }
    }
}

/// Error type for GIF processing operations
///
/// This enum provides specific error types for different failure scenarios
/// in the GIF processing pipeline, making error handling more precise.
#[derive(Debug)]
pub enum GifError {
    /// Error occurred during GIF decoding
    DecodeError(String),
    /// Error occurred during GIF encoding
    EncodeError(String),
    /// Error occurred during canvas operations
    CanvasError(String),
    /// Error due to invalid state or input
    InvalidState(String),
}

impl From<GifError> for JsValue {
    fn from(error: GifError) -> Self {
        JsValue::from_str(&error.to_string())
    }
}

impl std::fmt::Display for GifError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GifError::DecodeError(e) => write!(f, "GIF decode error: {}", e),
            GifError::EncodeError(e) => write!(f, "GIF encode error: {}", e),
            GifError::CanvasError(e) => write!(f, "Canvas error: {}", e),
            GifError::InvalidState(e) => write!(f, "Invalid state: {}", e),
        }
    }
}

/// Processes GIF images and adds captions
///
/// This struct handles all GIF processing operations including:
/// - Loading and decoding GIF files
/// - Managing frame animation
/// - Adding text captions
/// - Composing frames with text overlays
/// - Encoding processed GIFs
#[wasm_bindgen]
pub struct GifProcessor {
    /// Raw frame data for each frame in the GIF
    frames: Vec<Vec<u8>>,
    /// Width of the GIF in pixels
    width: usize,
    /// Height of the GIF in pixels
    height: usize,
    /// Index of the current frame being displayed
    current_frame: usize,
    /// Delay time for each frame in centiseconds
    frame_delays: Vec<u16>,
}

#[wasm_bindgen]
impl GifProcessor {
    /// Creates a new GifProcessor instance
    ///
    /// Initializes all fields to their default values. The processor
    /// must be populated with GIF data using `process_gif` before use.
    #[wasm_bindgen(constructor)]
    pub fn new() -> GifProcessor {
        GifProcessor {
            frames: Vec::new(),
            width: 0,
            height: 0,
            current_frame: 0,
            frame_delays: Vec::new(),
        }
    }

    /// Process a GIF file and store its frames
    ///
    /// Decodes the provided GIF data and stores all frames and their timing information.
    /// This must be called before any other operations can be performed.
    ///
    /// # Arguments
    /// * `gif_data` - Raw bytes of the GIF file
    ///
    /// # Returns
    /// * `Result<(), JsValue>` - Ok if processing succeeded, Error if it failed
    #[wasm_bindgen]
    pub fn process_gif(&mut self, gif_data: &[u8]) -> Result<(), JsValue> {
        if gif_data.is_empty() {
            return Err(GifError::InvalidState("Empty GIF data provided".into()).into());
        }

        let cursor = Cursor::new(gif_data);
        let mut decoder = gif::DecodeOptions::new();
        decoder.set_color_output(gif::ColorOutput::RGBA);

        let mut decoder = decoder
            .read_info(cursor)
            .map_err(|e| GifError::DecodeError(e.to_string()))?;

        self.width = decoder.width() as usize;
        self.height = decoder.height() as usize;
        self.current_frame = 0;

        self.frames = Vec::with_capacity(100);
        self.frame_delays = Vec::with_capacity(100);

        let mut canvas = vec![0u8; self.width * self.height * 4];
        let mut previous_canvas = canvas.clone();

        while let Some(frame) = decoder
            .read_next_frame()
            .map_err(|e| GifError::DecodeError(e.to_string()))?
        {
            let frame_width = frame.width as usize;
            let frame_height = frame.height as usize;
            let frame_top = frame.top as usize;
            let frame_left = frame.left as usize;

            match frame.dispose {
                gif::DisposalMethod::Background => {
                    canvas.fill(0);
                }
                gif::DisposalMethod::Previous => {
                    canvas.copy_from_slice(&previous_canvas);
                }
                gif::DisposalMethod::Keep | gif::DisposalMethod::Any => {
                    previous_canvas.copy_from_slice(&canvas);
                }
            }

            for y in 0..frame_height {
                for x in 0..frame_width {
                    let src_idx = (y * frame_width + x) * 4;
                    let cx = frame_left + x;
                    let cy = frame_top + y;
                    if cx < self.width && cy < self.height {
                        let dst_idx = (cy * self.width + cx) * 4;
                        let pixel = &frame.buffer[src_idx..src_idx + 4];
                        if pixel[3] > 0 {
                            canvas[dst_idx..dst_idx + 4].copy_from_slice(pixel);
                        }
                    }
                }
            }

            self.frames.push(canvas.clone());
            self.frame_delays.push(frame.delay);
        }

        if self.frames.is_empty() {
            return Err(GifError::InvalidState("No frames found in GIF".into()).into());
        }

        Ok(())
    }

    /// Render the current frame to a canvas
    ///
    /// Draws the current frame to the provided HTML canvas element.
    /// The frame is scaled to fit the canvas dimensions.
    ///
    /// # Arguments
    /// * `canvas` - The HTML canvas element to draw on
    ///
    /// # Returns
    /// * `Result<(), JsValue>` - Ok if rendering succeeded, Error if it failed
    #[wasm_bindgen]
    pub fn render_current_frame(&self, canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
        if self.frames.is_empty() {
            return Err(GifError::InvalidState("No frames to render".into()).into());
        }

        let context = self.get_canvas_context(canvas)?;
        context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);

        let frame_data = &self.frames[self.current_frame];
        let image_data = ImageData::new_with_u8_clamped_array_and_sh(
            wasm_bindgen::Clamped(frame_data),
            self.width as u32,
            self.height as u32,
        )?;

        context.put_image_data(&image_data, 0.0, 0.0)?;

        Ok(())
    }

    /// Advance to the next frame
    ///
    /// Updates the current frame index to the next frame in the sequence.
    /// If there are no frames, this operation has no effect.
    #[wasm_bindgen]
    pub fn next_frame(&mut self) {
        if !self.frames.is_empty() {
            self.current_frame = (self.current_frame + 1) % self.frames.len();
        }
    }

    /// Add a caption to the current frame
    ///
    /// Renders text on top of the current frame with the specified styling.
    /// The text is rendered with a white fill and black outline for visibility.
    ///
    /// # Arguments
    /// * `canvas` - The HTML canvas element to draw on
    /// * `text` - The text to render
    /// * `x` - X coordinate for text placement
    /// * `y` - Y coordinate for text placement
    /// * `font_size` - Size of the font in pixels
    /// * `font_family` - Font family to use
    ///
    /// # Returns
    /// * `Result<(), JsValue>` - Ok if caption was added successfully, Error if it failed
    #[wasm_bindgen]
    pub fn add_caption(
        &self,
        canvas: &HtmlCanvasElement,
        text: &str,
        x: f64,
        y: f64,
        font_size: f64,
        font_family: &str,
    ) -> Result<(), JsValue> {
        if text.is_empty() {
            return Ok(());
        }

        let context = self.get_canvas_context(canvas)?;
        self.setup_text_style(&context, font_size, font_family)?;

        context.stroke_text(text, x, y)?;
        context.fill_text(text, x, y)?;

        Ok(())
    }

    /// Get the dimensions of the GIF
    ///
    /// Returns the width and height of the GIF in pixels.
    ///
    /// # Returns
    /// * `Dimensions` - Struct containing width and height
    #[wasm_bindgen]
    pub fn get_dimensions(&self) -> Dimensions {
        Dimensions {
            width: self.width,
            height: self.height,
        }
    }

    /// Get the delay of the current frame
    ///
    /// Returns the delay time for the current frame in centiseconds.
    /// If no frame is loaded, returns a default delay of 100 centiseconds.
    ///
    /// # Returns
    /// * `u16` - Frame delay in centiseconds
    #[wasm_bindgen]
    pub fn get_frame_delay(&self) -> u16 {
        self.frame_delays
            .get(self.current_frame)
            .copied()
            .unwrap_or(100)
    }

    /// Prepare a text overlay for compositing
    ///
    /// Creates a transparent canvas with the specified text rendered on it.
    /// This is used as a mask for compositing text onto GIF frames.
    ///
    /// # Arguments
    /// * `text` - The text to render
    /// * `x` - X coordinate for text placement
    /// * `y` - Y coordinate for text placement
    /// * `font_size` - Size of the font in pixels
    /// * `font_family` - Font family to use
    ///
    /// # Returns
    /// * `Result<Vec<u8>, JsValue>` - Ok with RGBA pixel data if successful, Error if it failed
    #[wasm_bindgen]
    pub fn prepare_text_overlay(
        &self,
        text: &str,
        x: f64,
        y: f64,
        font_size: f64,
        font_family: &str,
    ) -> Result<Vec<u8>, JsValue> {
        if text.is_empty() {
            return Ok(vec![0; self.width as usize * self.height as usize * 4]);
        }

        let document = web_sys::window()
            .ok_or_else(|| GifError::CanvasError("Failed to get window".into()))?
            .document()
            .ok_or_else(|| GifError::CanvasError("Failed to get document".into()))?;

        let canvas = document
            .create_element("canvas")?
            .dyn_into::<HtmlCanvasElement>()?;
        canvas.set_width(self.width as u32);
        canvas.set_height(self.height as u32);

        let context = self.get_canvas_context(&canvas)?;
        context.clear_rect(0.0, 0.0, self.width as f64, self.height as f64);
        self.setup_text_style(&context, font_size, font_family)?;

        context.stroke_text(text, x, y)?;
        context.fill_text(text, x, y)?;

        let text_data = context
            .get_image_data(0.0, 0.0, self.width as f64, self.height as f64)?
            .data()
            .to_vec();

        Ok(text_data)
    }

    /// Process all frames with the given text overlay
    ///
    /// Creates a new GIF with the text overlay composited onto each frame.
    /// The resulting GIF maintains the original timing and dimensions.
    ///
    /// # Arguments
    /// * `text_data` - RGBA pixel data of the text overlay
    ///
    /// # Returns
    /// * `Result<Vec<u8>, JsValue>` - Ok with the new GIF data if successful, Error if it failed
    #[wasm_bindgen]
    pub fn process_all_frames_with_text_data(&self, text_data: &[u8]) -> Result<Vec<u8>, JsValue> {
        if self.frames.is_empty() {
            return Err(GifError::InvalidState("No frames to process".into()).into());
        }

        let mut output = Vec::with_capacity(self.frames.len() * self.width * self.height);
        {
            let mut encoder = Encoder::new(&mut output, self.width as u16, self.height as u16, &[])
                .map_err(|e| GifError::EncodeError(e.to_string()))?;

            encoder
                .set_repeat(Repeat::Infinite)
                .map_err(|e| GifError::EncodeError(e.to_string()))?;

            for (i, frame_data) in self.frames.iter().enumerate() {
                let mut modified_data = self.composite_text_overlay(frame_data, text_data);

                let mut frame =
                    Frame::from_rgba(self.width as u16, self.height as u16, &mut modified_data);
                frame.delay = self.frame_delays[i];
                frame.dispose = gif::DisposalMethod::Keep;

                encoder
                    .write_frame(&frame)
                    .map_err(|e| GifError::EncodeError(e.to_string()))?;
            }
        }
        Ok(output)
    }

    /// Get the 2D rendering context from a canvas
    ///
    /// # Arguments
    /// * `canvas` - The HTML canvas element
    ///
    /// # Returns
    /// * `Result<CanvasRenderingContext2d, JsValue>` - Ok with the context if successful, Error if it failed
    fn get_canvas_context(
        &self,
        canvas: &HtmlCanvasElement,
    ) -> Result<CanvasRenderingContext2d, JsValue> {
        canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("Failed to get 2d context"))?
            .dyn_into::<CanvasRenderingContext2d>()
            .map_err(|_| JsValue::from_str("Failed to get 2d context"))
    }

    /// Set up text rendering style for the canvas context
    ///
    /// # Arguments
    /// * `context` - The canvas rendering context
    /// * `font_size` - Size of the font in pixels
    /// * `font_family` - Font family to use
    ///
    /// # Returns
    /// * `Result<(), JsValue>` - Ok if successful, Error if it failed
    fn setup_text_style(
        &self,
        context: &CanvasRenderingContext2d,
        font_size: f64,
        font_family: &str,
    ) -> Result<(), JsValue> {
        context.set_font(&format!("bold {}px {}", font_size, font_family));
        context.set_fill_style_str("white");
        context.set_stroke_style_str("black");
        context.set_line_width(3.0);
        Ok(())
    }

    /// Composite text overlay with frame data
    ///
    /// Combines the text overlay with the frame data using alpha compositing.
    /// Only pixels with non-zero alpha in the text overlay are used.
    ///
    /// # Arguments
    /// * `frame_data` - RGBA pixel data of the frame
    /// * `text_data` - RGBA pixel data of the text overlay
    ///
    /// # Returns
    /// * `Vec<u8>` - Combined RGBA pixel data
    fn composite_text_overlay(&self, frame_data: &[u8], text_data: &[u8]) -> Vec<u8> {
        let mut result = frame_data.to_vec();
        for i in (0..text_data.len()).step_by(4) {
            if text_data[i + 3] > 0 {
                result[i] = text_data[i]; // R
                result[i + 1] = text_data[i + 1]; // G
                result[i + 2] = text_data[i + 2]; // B
                result[i + 3] = text_data[i + 3]; // A
            }
        }
        result
    }
}
