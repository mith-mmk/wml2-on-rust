// Trusted evaluation jobs only. No network, WASI or browser glue.
import fs from 'node:fs';
const [wasmPath, jobsPath] = process.argv.slice(2);
const module = await WebAssembly.compile(fs.readFileSync(wasmPath));
const instance = await WebAssembly.instantiate(module, {});
const api = instance.exports;
const results = [];
for (const job of JSON.parse(fs.readFileSync(jobsPath, 'utf8'))) {
  const input = fs.readFileSync(job.input);
  const p = api.probe_alloc(input.length);
  new Uint8Array(api.memory.buffer, p, input.length).set(input);
  const result = api.probe_encode(p, input.length, job.width, job.height, job.channels, job.bits);
  if (!result) throw Error(`encode failed: ${job.name}`);
  const pointer = api.probe_output_ptr(result);
  const length = api.probe_output_len(result);
  fs.writeFileSync(job.output, new Uint8Array(api.memory.buffer, pointer, length));
  api.probe_output_free(result);
  api.probe_free(p, input.length);
  results.push({ name: job.name, output_bytes: length, linear_memory_bytes: api.memory.buffer.byteLength });
}
console.log(JSON.stringify({ node: process.version, imports: WebAssembly.Module.imports(module), results }));
