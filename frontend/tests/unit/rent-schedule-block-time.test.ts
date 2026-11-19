import { describe, expect, it, vi } from 'vitest';
import type { RentSchedulePageDto } from '../../src/lib/api/types';

// A 30-second-block build (YOLO). The mock applies to every module importing `$lib/chain`.
vi.mock('$lib/chain', async (importOriginal) => ({
	...(await importOriginal<typeof import('$lib/chain')>()),
	BLOCK_SECONDS: 30
}));

const { validateSchedule } = await import('../../src/lib/rent/schedule');
const { ENDPOINTS } = await import('../../src/lib/developer/playground');

const id = (n: number) => n.toString(16).padStart(64, '0');
const anchor = { height: 200, block_id: id(10000) };
const time = Date.UTC(2026, 9, 8);
const query = { window: '24h', mode: 'all' } as const;

function page(estimated: number): RentSchedulePageDto {
	return {
		items: [
			{
				box_id: id(1),
				tx_id: id(10001),
				creation_height: 0,
				maturity_height: 203,
				estimated_maturity_ms: estimated,
				value: '1000000',
				consensus_fee_nano: '125000000',
				collectible_due_nano: '1000000',
				collectible: true,
				full_claim: true,
				protocol_constrained: false,
				tokens: []
			}
		],
		next_cursor: null,
		next_snapshot: null,
		consistency: 'strict',
		anchor,
		observed_anchor: anchor,
		schedule_context: {
			scope: 'indexed_unspent_rent_window',
			anchor_timestamp_ms: time,
			from_height: 201,
			to_height: 3080,
			target_block_seconds: 30,
			window: '24h',
			mode: 'all',
			token_id: null,
			full_history: true,
			partial_from: null,
			scanned: 1,
			scan_limit: 2000,
			scan_limit_reached: false,
			complete: true,
			batch_scanned: 1000,
			batch_scan_limit: 50_000,
			batch_complete: true,
			batch_stop_reason: null,
			rent_factor: 300_000,
			fee_arithmetic: 'wrapping_i32'
		},
		batches: [
			{
				estimated_hour_start_ms: time,
				box_count: 1,
				collectible_due_nano: '1000000',
				full_claim_count: 1,
				full_claim_value_nano: '1000000',
				selected_token_full_claim_amount: null
			}
		]
	};
}

describe('rent schedule on a 30-second chain', () => {
	it('checks estimated maturity against the build block time, not 120 s', () => {
		// Three blocks past the anchor.
		const data = page(time + 3 * 30_000);
		expect(validateSchedule(data, query)).toBe(data);
		expect(() => validateSchedule(page(time + 3 * 120_000), query)).toThrow('inconsistent');
	});

	it('describes the block target in the playground note', () => {
		const note = ENDPOINTS.find((endpoint) => endpoint.id === 'rent-schedule')?.note ?? '';
		expect(note).toContain('30-second block target');
		expect(note).not.toContain('120-second');
	});
});
