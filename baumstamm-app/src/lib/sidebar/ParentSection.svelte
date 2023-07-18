<script lang="ts">
	import { type Relationship, addParent } from '../../bindings';
	import { persons, update } from '$lib/store';

	export let parentRel: Relationship;

	function newParent() {
		addParent(parentRel.id);
		update();
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
					<tr>
						<td class="table-cell-fit">{parent?.name()}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
	{#if parentRel.parents.filter((parent) => parent !== null).length < 2}
		<button on:click={newParent} type="button" class="btn variant-filled m-1">Add Parent</button>
	{/if}
</section>
