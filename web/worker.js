import init, { describe, embed, extract, version } from './pkg/iwm_web.js';

const ready = init();

const methods = {
  describe(bytes) {
    const description = describe(bytes);
    const size = {
      width: description.width,
      height: description.height,
      capacity: description.capacity,
      capacityWithPassword: description.capacity_with_password,
      resilientCapacity: description.resilient_capacity,
    };
    description.free();
    return size;
  },
  embed,
  extract,
  version,
};

self.addEventListener('message', async ({ data: { id, method, args } }) => {
  try {
    await ready;
    const value = methods[method](...args);
    self.postMessage({ id, value }, value instanceof Uint8Array ? [value.buffer] : []);
  } catch (error) {
    // A trap can leave the module's memory inconsistent, so this worker is not used again.
    if (error instanceof WebAssembly.RuntimeError) {
      self.postMessage({ id, error: `the WebAssembly module crashed (${error.message})`, closed: true });
      self.close();
    } else {
      self.postMessage({ id, error: error instanceof Error ? error.message : String(error) });
    }
  }
});
