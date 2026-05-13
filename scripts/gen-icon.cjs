// Generate a minimal 32x32 PNG placeholder so `tauri::generate_context!`
// has something to load. Replace with a real icon before shipping.

const fs = require('fs');
const path = require('path');
const zlib = require('zlib');

let table;
function crc32(buf) {
  if (!table) {
    table = new Uint32Array(256);
    for (let n = 0; n < 256; n++) {
      let c = n;
      for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      table[n] = c >>> 0;
    }
  }
  let crc = 0xffffffff;
  for (let i = 0; i < buf.length; i++) crc = (table[(crc ^ buf[i]) & 0xff] ^ (crc >>> 8)) >>> 0;
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const typeBuf = Buffer.from(type, 'ascii');
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])), 0);
  return Buffer.concat([len, typeBuf, data, crc]);
}

function makePng(width, height, rgba) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  // bit depth 8, color type 6 (RGBA), no compression, no filter, no interlace
  ihdr[8] = 8; ihdr[9] = 6; ihdr[10] = 0; ihdr[11] = 0; ihdr[12] = 0;

  const stride = width * 4;
  const raw = Buffer.alloc((stride + 1) * height);
  for (let y = 0; y < height; y++) {
    raw[y * (stride + 1)] = 0;
    for (let x = 0; x < width; x++) {
      const o = y * (stride + 1) + 1 + x * 4;
      raw[o] = rgba[0];
      raw[o + 1] = rgba[1];
      raw[o + 2] = rgba[2];
      raw[o + 3] = rgba[3] ?? 0xff;
    }
  }
  const idat = zlib.deflateSync(raw);
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', idat),
    chunk('IEND', Buffer.alloc(0))
  ]);
}

const iconsDir = path.join(__dirname, '..', 'src-tauri', 'icons');
fs.mkdirSync(iconsDir, { recursive: true });

const COLOR = [0x25, 0x63, 0xeb, 0xff]; // accent blue, opaque
for (const size of [32, 128, 256, 512]) {
  const png = makePng(size, size, COLOR);
  fs.writeFileSync(path.join(iconsDir, `${size}x${size}.png`), png);
}
fs.writeFileSync(path.join(iconsDir, 'icon.png'), makePng(512, 512, COLOR));
console.log('icons written to', iconsDir);
