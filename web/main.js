const ENCRYPTED = 'watermark is encrypted, a password is required';

const pending = new Map();
let nextId = 0;
let worker = null;
let workerError = null;

const fileInput = document.getElementById('file');
const imageInfo = document.getElementById('image-info');
const imageStatus = document.getElementById('image-status');
const embedForm = document.getElementById('embed-form');
const embedText = document.getElementById('embed-text');
const embedPassword = document.getElementById('embed-password');
const embedButton = document.getElementById('embed-button');
const embedStatus = document.getElementById('embed-status');
const embedResult = document.getElementById('embed-result');
const embedPreview = document.getElementById('embed-preview');
const embedDownload = document.getElementById('embed-download');
const readForm = document.getElementById('read-form');
const readPassword = document.getElementById('read-password');
const readButton = document.getElementById('read-button');
const readStatus = document.getElementById('read-status');
const readText = document.getElementById('read-text');
const versionText = document.getElementById('version');
const readBack = document.getElementById('read-back');

let image = null;
let working = false;
let resultUrl = null;
let result = null;

function startWorker() {
  worker = new Worker(new URL('worker.js', import.meta.url), { type: 'module' });
  worker.addEventListener('message', settle);
  worker.addEventListener('error', failToLoad);
}

function call(method, args = [], transfer = []) {
  if (workerError !== null) {
    return Promise.reject(workerError);
  }
  if (worker === null) {
    startWorker();
  }
  return new Promise((resolve, reject) => {
    const id = nextId++;
    pending.set(id, { resolve, reject });
    worker.postMessage({ id, method, args }, transfer);
  });
}

function rejectPending(error) {
  for (const { reject } of pending.values()) {
    reject(error);
  }
  pending.clear();
}

function settle({ data }) {
  const { resolve, reject } = pending.get(data.id);
  pending.delete(data.id);
  if ('error' in data) {
    reject(new Error(data.error));
  } else {
    resolve(data.value);
  }
  if (data.closed) {
    worker = null;
    rejectPending(new Error(data.error));
  }
}

function failToLoad(event) {
  workerError = new Error(event.message || 'cannot start the worker');
  rejectPending(workerError);
}

async function readBytes(file) {
  return new Uint8Array(await file.arrayBuffer());
}

function updateControls() {
  embedButton.disabled = working || image === null;
  readButton.disabled = working || image === null;
  readBack.disabled = working;
  fileInput.disabled = working;
}

async function run(status, task) {
  const focused = document.activeElement;
  working = true;
  updateControls();
  status.textContent = 'Working...';
  try {
    status.textContent = await task();
  } catch (error) {
    status.textContent = `Error: ${error.message}`;
  } finally {
    working = false;
    updateControls();
    // A control disabled while working loses focus; give it back unless the task moved focus.
    if (document.activeElement === document.body) {
      focused.focus();
    }
  }
}

function count(n) {
  return n.toLocaleString('en-US');
}

function stem(name) {
  const dot = name.lastIndexOf('.');
  return dot > 0 ? name.slice(0, dot) : name;
}

async function describeFile(file) {
  const bytes = await readBytes(file);
  const { width, height, capacity, capacityWithPassword } = await call('describe', [bytes], [bytes.buffer]);
  return `${file.name}: ${width} × ${height} pixels. Holds up to ${count(capacity)} bytes of text, or ${count(capacityWithPassword)} with a password.`;
}

function forgetResult() {
  if (resultUrl !== null) {
    URL.revokeObjectURL(resultUrl);
    resultUrl = null;
  }
  result = null;
  embedResult.hidden = true;
  embedPreview.removeAttribute('src');
  embedDownload.removeAttribute('href');
}

async function choose(file) {
  image = null;
  forgetResult();
  readText.hidden = true;
  imageInfo.hidden = true;
  embedStatus.textContent = '';
  readStatus.textContent = '';
  if (file === null) {
    imageStatus.textContent = '';
    updateControls();
    return;
  }
  await run(imageStatus, async () => {
    imageInfo.textContent = await describeFile(file);
    imageInfo.hidden = false;
    image = file;
    return '';
  });
}

async function embedImage(event) {
  event.preventDefault();
  forgetResult();
  await run(embedStatus, async () => {
    const bytes = await readBytes(image);
    const png = await call('embed', [bytes, embedText.value, embedPassword.value], [bytes.buffer]);
    const name = `${stem(image.name)}.wm.png`;
    result = new File([png], name, { type: 'image/png' });
    resultUrl = URL.createObjectURL(result);
    embedPreview.src = resultUrl;
    embedDownload.href = resultUrl;
    embedDownload.download = name;
    embedDownload.textContent = `Download ${name}`;
    embedResult.hidden = false;
    embedDownload.focus();
    return '';
  });
}

async function readBackResult() {
  const file = result;
  await run(embedStatus, async () => {
    imageInfo.textContent = await describeFile(file);
    imageInfo.hidden = false;
    image = file;
    fileInput.value = '';
    readStatus.textContent = '';
    readText.hidden = true;
    readPassword.focus();
    return '';
  });
}

async function readImage(event) {
  event.preventDefault();
  readText.hidden = true;
  await run(readStatus, async () => {
    const bytes = await readBytes(image);
    try {
      readText.textContent = await call('extract', [bytes, readPassword.value], [bytes.buffer]);
      readText.hidden = false;
      readText.focus();
      return '';
    } catch (error) {
      if (error.message === ENCRYPTED) {
        readPassword.focus();
        return 'This watermark is encrypted. Enter the password and press Read.';
      }
      throw error;
    }
  });
}

function carriesFiles(event) {
  return event.dataTransfer.types.includes('Files');
}

function allowDrop(event) {
  if (!carriesFiles(event)) {
    return;
  }
  event.preventDefault();
  event.dataTransfer.dropEffect = working ? 'none' : 'copy';
}

function dropFile(event) {
  if (!carriesFiles(event)) {
    return;
  }
  event.preventDefault();
  const file = event.dataTransfer.files[0];
  if (working || !file) {
    return;
  }
  const transfer = new DataTransfer();
  transfer.items.add(file);
  fileInput.files = transfer.files;
  choose(file);
}

fileInput.addEventListener('change', () => choose(fileInput.files[0] ?? null));
embedForm.addEventListener('submit', embedImage);
readForm.addEventListener('submit', readImage);
readBack.addEventListener('click', readBackResult);
document.addEventListener('dragover', allowDrop);
document.addEventListener('drop', dropFile);

call('version').then(
  (version) => {
    versionText.textContent = `Version ${version}`;
  },
  (error) => {
    imageStatus.textContent = `Error: ${error.message}`;
  },
);
