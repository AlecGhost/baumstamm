<script lang="ts">
	import type { Person } from "$lib/Person";
	import { update } from "$lib/store";
	import { insertInfo, removeInfo } from "../../bindings";
    import { focusTrap } from "@skeletonlabs/skeleton";

    export let person: Person;

	let isFocused: boolean = true;
	let infoKey = '';
	let infoValue = '';

	function submitInfo() {
		if (person !== null) {
			insertInfo(person.id, infoKey, infoValue);
			update();
			person.info.set(infoKey, infoValue);
			// let svelte know, that the value updated
			person = person;
		}
		infoKey = '';
		infoValue = '';
	}

	function deleteInfo() {
		if (person !== null) {
			removeInfo(person.id, infoKey);
			update();
			person.info.delete(infoKey);
			// let svelte know, that the value updated
			person = person;
		}
		infoKey = '';
		infoValue = '';
	}
</script>

<section class="p-4">
	<div class="table-container m-1">
		<table class="table table-hover">
			<tbody>
				{#each person.info as [key, value]}
					<tr
						on:click={() => {
							infoKey = key;
							infoValue = value;
						}}
					>
						<td class="table-cell-fit">{key}</td>
						<td class="table-cell-fit">{value}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
	<form on:submit|preventDefault={submitInfo} use:focusTrap={isFocused}>
		<input type="text" placeholder="Key" class="input m-1" bind:value={infoKey} />
		<input type="text" placeholder="Value" class="input m-1" bind:value={infoValue} />
		<button type="submit" class="btn variant-filled-primary m-1">Add</button>
		{#if person.info.has(infoKey)}
			<button on:click={deleteInfo} type="button" class="btn variant-filled-error m-1"
				>Delete</button
			>
		{/if}
	</form>
</section>
