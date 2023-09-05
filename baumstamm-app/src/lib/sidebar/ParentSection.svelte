<script lang="ts">
	import { type Relationship, addParent } from '../../bindings';
	import { persons, selected, update, updateSelected } from '$lib/store';

	export let parentRel: Relationship;

	async function newParent() {
		let [pid, _] = await addParent(parentRel.id);
		await update();
		updateSelected(pid);
	}
</script>

<section class="p-4">
	<span class="label">Parents:</span>
	<div class="table-container m-1">
		<table class="table table-hover">
			<tbody>
				{#each parentRel.parents
					.filter((parent) => parent !== null)
					.map((pid) => $persons.find((person) => person.id == pid)) as parent}
					<tr on:click={() => $selected = parent ?? null} class="cursor-pointer">
						<td class="table-cell-fit">{parent?.fullName()}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
	{#if parentRel.parents.filter((parent) => parent !== null).length < 2}
		<button on:click={newParent} type="button" class="btn variant-filled m-1">Add Parent</button>
	{/if}
</section>
