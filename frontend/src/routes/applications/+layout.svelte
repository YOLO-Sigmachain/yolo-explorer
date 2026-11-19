<script lang="ts">
	import type { Snippet } from 'svelte';
	import { CHAIN_NAME, ERGO_APPS } from '$lib/chain';

	let { children }: { children: Snippet } = $props();
</script>

<svelte:head>
	{#if !ERGO_APPS}<title>Not available — {CHAIN_NAME} Explorer</title>{/if}
</svelte:head>

{#if ERGO_APPS}
	{@render children()}
{:else}
	<p>Applications are not available on this chain.</p>
{/if}
