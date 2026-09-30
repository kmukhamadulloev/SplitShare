import assert from 'node:assert/strict'
import { aggregate } from '../src/app/aggregate.ts'
const batch = [
  { total: 100, transferred: 75, state: 'uploading' },
  { total: 300, transferred: 125, state: 'uploading' },
  { total: 100, transferred: 0, state: 'queued' },
]
assert.deepEqual(aggregate(batch), { transferred: 200, total: 500, percent: 40, active: 2, queued: 1 })
assert.equal(aggregate([{ total: null, transferred: 10, state: 'uploading' }]).percent, null)
assert.equal(aggregate([{ total: 0, transferred: 0, state: 'queued' }]).percent, 0)
assert.equal(aggregate([{ total: 0, transferred: 0, state: 'completed' }]).percent, 100)
assert.equal(aggregate([{ total: 100, transferred: 25, state: 'cancelled' }]).percent, 25)
console.log('PASS: aggregate weighted bytes, unknown/zero totals, cancellation')
