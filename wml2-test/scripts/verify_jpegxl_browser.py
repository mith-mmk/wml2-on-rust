#!/usr/bin/env python3
"""Build portable WASM and display verified JPEG XL pixels on Chromium canvas."""
import argparse,base64,hashlib,json,pathlib,re,subprocess,shutil,zlib
p=argparse.ArgumentParser();p.add_argument('--output',type=pathlib.Path,required=True);p.add_argument('--browser',default='chromium');a=p.parse_args()
if not a.output.name.startswith('.test'):p.error('--output must start with .test')
root=pathlib.Path(__file__).resolve().parents[2];out=a.output.resolve();out.mkdir(parents=True,exist_ok=True);target=out/'.test-target';features='jpeg-xl,image-buffer-ex,color-management'
def run(args,log):
 r=subprocess.run([str(v) for v in args],cwd=root,capture_output=True,text=True);(out/log).write_text(r.stdout+r.stderr);r.check_returncode();return r.stdout
run(['cargo','build','-p','wml2','--locked','--release','--target','wasm32-unknown-unknown','--target-dir',target,'--no-default-features','--features',features],'build.log')
deps=target/'wasm32-unknown-unknown/release';wasm=out/'jpegxl-browser.wasm'
run(['rustc','--edition','2024','--target','wasm32-unknown-unknown','--crate-type','cdylib','-C','opt-level=3','-C','panic=abort',root/'wml2-test/scripts/jpegxl_browser.rs','--extern',f'wml2={deps/"libwml2.rlib"}','--extern','miniz_oxide='+str(next((deps/'deps').glob('libminiz_oxide-*.rlib'))),'-L',f'dependency={deps/"deps"}','-L',f'dependency={target/"release/deps"}','-o',wasm],'harness.log')
manifest=json.loads((root/'wml2/tests/fixtures/review/jxl-additional-fixtures.json').read_text());cases=[]
for item in manifest['orientations']+manifest['managed']:
 reference=(root/'wml2/tests/fixtures/review'/item['expected']).read_bytes()
 if item.get('compression')=='zlib':reference=zlib.decompress(reference)
 cases.append({'name':item['file'],'reference':base64.b64encode(reference).decode()})
html='''<!doctype html><meta charset="utf-8"><title>JPEG XL WASM display verification</title>
<style>body{font:16px sans-serif;background:#f4f4f4}#grid{display:grid;grid-template-columns:repeat(3,310px);gap:16px}.card{background:white;padding:12px}canvas{width:120px;height:120px;image-rendering:pixelated;background:repeating-conic-gradient(#ccc 0% 25%,white 0% 50%) 50% / 16px 16px}h2{font-size:13px}</style>
<h1>JPEG XL: independent reference / WML2 WASM</h1><p id="status">Checking...</p><div id="grid"></div><pre id="result"></pre>
<script>
const bytes=Uint8Array.from(atob(WASM),c=>c.charCodeAt(0));const cases=CASES;
(async()=>{const module=await WebAssembly.compile(bytes);const imports=WebAssembly.Module.imports(module);if(imports.length)throw Error('unexpected imports');const {exports:e}=await WebAssembly.instantiate(module,{});if(e.case_count()!==cases.length)throw Error('case count');let maxDelta=0;
for(let i=0;i<cases.length;i++){const ptr=e.render_case(i),w=e.width(),h=e.height();const actual=new Uint8ClampedArray(e.memory.buffer,ptr,w*h*4).slice();const reference=Uint8ClampedArray.from(atob(cases[i].reference),c=>c.charCodeAt(0));if(actual.length!==reference.length)throw Error('length');let delta=0;for(let j=0;j<actual.length;j++)delta=Math.max(delta,Math.abs(actual[j]-reference[j]));if(delta>(i<8?0:2))throw Error('pixel difference');maxDelta=Math.max(maxDelta,delta);
const card=document.createElement('div');card.className='card';const title=document.createElement('h2');title.textContent=cases[i].name+' (max '+delta+')';card.append(title);for(const rgba of [reference,actual]){const canvas=document.createElement('canvas');canvas.width=w;canvas.height=h;canvas.getContext('2d').putImageData(new ImageData(rgba,w,h),0,0);card.append(canvas)}document.getElementById('grid').append(card)}
const report={all_passed:true,cases:cases.length,orientation_cases:8,managed_cases:cases.length-8,max_rgba_difference:maxDelta,host_imports:imports,memory_bytes:e.memory.buffer.byteLength};document.getElementById('status').textContent='All verified; left reference, right WML2';document.getElementById('result').textContent=JSON.stringify(report);document.body.dataset.result='passed';})().catch(e=>{document.getElementById('result').textContent=String(e);document.body.dataset.result='failed'});
</script>'''.replace('atob(WASM)','atob('+json.dumps(base64.b64encode(wasm.read_bytes()).decode())+')').replace('CASES',json.dumps(cases),1)
page=out/'jpegxl-display.html';page.write_text(html);screenshot=out/'jpegxl-display.png'
from playwright.sync_api import sync_playwright
with sync_playwright() as playwright:
 browser=playwright.chromium.launch_persistent_context(str(out/'.test-playwright-profile'),executable_path=shutil.which(a.browser) or a.browser,headless=True,viewport={'width':1000,'height':1050},args=['--no-sandbox','--disable-background-networking','--disable-component-update','--no-first-run'])
 try:
  page_view=browser.new_page();page_view.set_content(html,wait_until='load',timeout=30000)
  page_view.wait_for_function("['passed','failed'].includes(document.body.dataset.result)",timeout=30000)
  (out/'browser-result.txt').write_text(page_view.locator('#result').inner_text())
  if page_view.locator('body').get_attribute('data-result')!='passed':raise RuntimeError('browser verification failed; see browser-result.txt')
  report=json.loads(page_view.locator('#result').inner_text());page_view.screenshot(path=str(screenshot),full_page=True)
  report.update(wasm_sha256=hashlib.sha256(wasm.read_bytes()).hexdigest(),features=features,target='wasm32-unknown-unknown',browser=browser.browser.version,screenshot=str(screenshot))
 finally:browser.close()

(out/'report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
