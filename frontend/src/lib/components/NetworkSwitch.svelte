<script lang="ts">
	import { page } from '$app/state';
	import {
		CHAIN_NETWORK,
		NETWORK_LABEL,
		NETWORK_LINKS,
		currentNetworkIndex,
		networkHref
	} from '$lib/chain';

	// The network name as a disclosure button over a list of plain links: each link is the same
	// route on the other explorer, so it can be followed, opened in a new tab or copied.
	const uid = $props.id();
	let open = $state(false);
	let up = $state(false);
	let alignEnd = $state(false);
	let root: HTMLDivElement;
	let trigger: HTMLButtonElement;

	const current = $derived(currentNetworkIndex(NETWORK_LINKS, NETWORK_LABEL, page.url.origin));
	const hrefs = $derived(
		NETWORK_LINKS.map((link) =>
			networkHref(link.url, page.url.pathname, page.url.search, page.url.hash)
		)
	);

	$effect(() => {
		void page.url.pathname;
		open = false;
	});

	function toggle() {
		if (!open) {
			// The label sits at the top of the screen in some layouts and at the bottom in others,
			// so the list opens towards whichever side has room.
			const rect = trigger.getBoundingClientRect();
			up = rect.top > window.innerHeight / 2;
			alignEnd = rect.left > window.innerWidth / 2;
		}
		open = !open;
	}

	function keys(event: KeyboardEvent) {
		if (event.key !== 'Escape' || !open) return;
		open = false;
		trigger.focus();
	}

	function pointer(event: PointerEvent) {
		if (open && !root.contains(event.target as Node)) open = false;
	}

	function leave(event: FocusEvent) {
		if (open && !root.contains(event.relatedTarget as Node | null)) open = false;
	}
</script>

<svelte:window onkeydown={keys} onpointerdown={pointer} />

<div class="network-switch" bind:this={root} onfocusout={leave}>
	<button
		type="button"
		class="network-trigger"
		aria-expanded={open}
		aria-controls="{uid}-networks"
		bind:this={trigger}
		onclick={toggle}
	>
		<span class="visually-hidden">Network:</span>
		<span class="network-name">{CHAIN_NETWORK}</span>
		<span class="caret" aria-hidden="true"></span>
	</button>
	<ul
		class="network-menu"
		id="{uid}-networks"
		aria-label="Networks"
		hidden={!open}
		class:up
		class:end={alignEnd}
	>
		{#each NETWORK_LINKS as link, i (link.label)}
			<li>
				<a
					href={hrefs[i]}
					class:current={i === current}
					aria-current={i === current ? 'true' : undefined}>{link.label}</a
				>
			</li>
		{/each}
	</ul>
</div>

<style>
	.network-switch {
		position: relative;
		display: inline-flex;
		min-width: 0;
	}

	/* The trigger reads as the label it replaces: same font, colour and case as its container,
	   sized in em so a container that hides its text (font-size: 0) hides the trigger too. */
	.network-trigger {
		display: inline-flex;
		align-items: center;
		gap: 0.4em;
		min-width: 0;
		padding: 0;
		border: 0;
		border-radius: 4px;
		background: none;
		color: inherit;
		font: inherit;
		letter-spacing: inherit;
		text-transform: inherit;
		cursor: pointer;
	}

	.network-name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.network-trigger:hover .network-name {
		text-decoration: underline;
		text-underline-offset: 0.2em;
	}

	.caret {
		flex: none;
		width: 0;
		height: 0;
		border-left: 0.3em solid transparent;
		border-right: 0.3em solid transparent;
		border-top: 0.36em solid currentcolor;
	}

	.network-trigger[aria-expanded='true'] .caret {
		transform: rotate(180deg);
	}

	.network-menu {
		position: absolute;
		z-index: 60;
		top: calc(100% + 6px);
		left: 0;
		min-width: 160px;
		max-width: calc(100vw - 32px);
		margin: 0;
		padding: 6px;
		list-style: none;
		border: var(--rule);
		border-radius: var(--radius-control);
		background: var(--surface-solid);
		color: var(--fg);
		box-shadow: var(--shadow-lift);
		font-family: var(--font-sans);
		font-size: var(--fs-body);
		font-weight: 500;
		letter-spacing: normal;
		text-transform: none;
		text-align: left;
	}

	.network-menu.up {
		top: auto;
		bottom: calc(100% + 6px);
	}

	.network-menu.end {
		left: auto;
		right: 0;
	}

	.network-menu a {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		min-height: 44px;
		padding: 8px 12px;
		border-radius: var(--radius-control);
		color: var(--fg);
		white-space: nowrap;
	}

	.network-menu a:hover,
	.network-menu a:focus-visible {
		background: var(--surface-hover);
		color: var(--fg);
	}

	.network-menu a.current {
		font-weight: 700;
	}

	.network-menu a.current::after {
		content: '✓';
		color: var(--fg-muted);
	}

	@media (max-width: 899px) {
		.network-trigger {
			min-height: 44px;
		}
	}
</style>
