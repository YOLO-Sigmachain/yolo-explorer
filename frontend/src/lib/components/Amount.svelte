<script lang="ts">
	import { formatErg, formatNano } from '$lib/format/amount';
	import { NANO_UNIT, TICKER } from '$lib/chain';

	interface Props {
		nano: string;
		unit?: string;
		maxFrac?: number;
	}

	let { nano, unit = TICKER, maxFrac }: Props = $props();

	const text = $derived(unit !== NANO_UNIT ? formatErg(nano, { maxFrac }) : formatNano(nano));
</script>

<span class="mono amount" title={`${nano} ${NANO_UNIT}`}
	>{text}<span class="unit">{unit}</span></span
>

<style>
	.amount {
		white-space: nowrap;
	}
	.unit {
		color: var(--fg-muted);
		font-size: 11px;
		margin-left: var(--space-1);
	}
</style>
