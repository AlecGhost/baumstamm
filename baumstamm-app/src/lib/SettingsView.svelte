<script lang="ts">
	import { SlideToggle } from '@skeletonlabs/skeleton';
	import { settings } from '$lib/store';

	export let show: boolean;
</script>

{#if show}
	<div class="p-4">
		<h1 class="p-4">Settings</h1>
		<hr />
		<section class="p-4">
			<span>Display in family tree</span>
			<div>
				<SlideToggle name="avatar" bind:checked={$settings.showAvatar}>Images</SlideToggle>
			</div>
			<div>
				<SlideToggle name="names" bind:checked={$settings.showNames}>Names</SlideToggle>
			</div>
		</section>
	</div>
{/if}
