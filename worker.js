// Import the wasm module
import init, { GifProcessor } from './pkg/gif_caption_wasm.js';

let gifProcessor = null;

// Initialize the wasm module
async function initWasm() {
    await init();
    gifProcessor = new GifProcessor();
}

// Handle messages from the main thread
self.onmessage = async function (e) {
    if (!gifProcessor) {
        await initWasm();
    }

    const { type, data } = e.data;

    switch (type) {
        case 'process_gif':
            try {
                gifProcessor.process_gif(data.gifData);
                const result = gifProcessor.process_all_frames_with_text_data(data.textData);
                self.postMessage({ type: 'gif_processed', data: result });
            } catch (error) {
                self.postMessage({ type: 'error', error: error.toString() });
            }
            break;
    }
};