<script lang="ts">
	import { BLOCKS_PER_DAY, TICKER } from '$lib/chain';
	import Badge from './Badge.svelte';
	import { formatErg } from '$lib/format/amount';
	import type { RentDto } from '$lib/api/types';

	/** Blocks-to-maturity at or under which a box is flagged as approaching (~1 day). */
	const SOON_BLOCKS = BLOCKS_PER_DAY;

	interface Props {
		rent: RentDto;
		tip: number | null;
	}

	let { rent, tip }: Props = $props();

	const blocksLeft = $derived(tip === null ? null : rent.maturity_height - tip);

	const tone = $derived.by((): 'neutral' | 'warn' | 'danger' => {
		if (rent.claimable_at_tip) return 'danger';
		if (blocksLeft !== null && blocksLeft <= SOON_BLOCKS) return 'warn';
		return 'neutral';
	});

	const label = $derived(
		rent.claimable_at_tip
			? 'claimable'
			: blocksLeft === null
				? `matures at ${rent.maturity_height}`
				: `matures in ${blocksLeft} blocks`
	);

	const title = $derived(
		`Maturity height ${rent.maturity_height} · due ${formatErg(rent.due_nano)} ${TICKER}`
	);
</script>

<Badge {tone} {title}>{label}</Badge>
