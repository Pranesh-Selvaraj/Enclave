import * as assert from 'node:assert';
import { mindmapChildPos, centerInside, orderFrames, fitCam } from '../src/lib/wbLayout';

const parent = { x: 100, y: 200, w: 140, h: 44 };
const first = mindmapChildPos(parent, []);
assert.deepStrictEqual(first, { x: 264, y: 200 }, 'first child: right of parent, same top');
const child1 = { x: 264, y: 200, w: 140, h: 44 };
const second = mindmapChildPos(parent, [child1]);
assert.deepStrictEqual(second, { x: 264, y: 268 }, 'second child stacks below the first');
const third = mindmapChildPos(parent, [child1, { x: 264, y: 268, w: 140, h: 60 }]);
assert.deepStrictEqual(third, { x: 264, y: 352 }, 'stacking uses the tallest sibling bottom');
console.log('mindmapChildPos: PASS');

const frame = { x: 0, y: 0, w: 400, h: 240 };
assert.ok(centerInside({ x: 100, y: 100, w: 20, h: 20 }, frame), 'element center inside');
assert.ok(!centerInside({ x: 390, y: 230, w: 40, h: 40 }, frame), 'center at edge outside');
assert.ok(!centerInside({ x: 500, y: 100, w: 20, h: 20 }, frame), 'element fully outside');
console.log('centerInside: PASS');

const frames = [
	{ x: 300, y: 100, w: 100, h: 100 },
	{ x: 0, y: 50, w: 100, h: 100 },
	{ x: 100, y: 0, w: 100, h: 100 },
];
assert.deepStrictEqual(
	orderFrames(frames).map((f) => f.y),
	[0, 50, 100],
	'frames ordered top-to-bottom, then left-to-right'
);
assert.deepStrictEqual(
	orderFrames(frames)[0],
	{ x: 100, y: 0, w: 100, h: 100 },
	'topmost first'
);
console.log('orderFrames: PASS');

const cam = fitCam({ x: 50, y: 50, w: 400, h: 240 }, 800, 600);
assert.ok(Math.abs(cam.zoom - 1.84) < 1e-9, `fits width: zoom ${cam.zoom}`);
assert.ok(cam.x > 0 && cam.y > 0, 'centers the frame in the viewport');
const camTiny = fitCam({ x: 0, y: 0, w: 10, h: 10 }, 800, 600);
assert.ok(Math.abs(camTiny.zoom - 4) < 1e-9, 'zoom clamped to max 4');
console.log('fitCam: PASS');

console.log('\n=== All wbLayout checks passed ===');
