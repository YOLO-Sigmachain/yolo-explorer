<script lang="ts">
	import { CHAIN_LOGO, CHAIN_MARK } from '$lib/chain';
</script>

<!-- The chain's mark: a picture when the build sets VITE_CHAIN_LOGO, else the one-character mark.
     The picture is sized in em, so it follows the font size of wherever the mark sits. -->
{#if CHAIN_LOGO}<img class="chain-logo" src={CHAIN_LOGO} alt="" />{:else}{CHAIN_MARK}{/if}

<style>
	.chain-logo {
		display: inline-block;
		width: auto;
		height: 1.25em;
		vertical-align: middle;
	}
</style>
