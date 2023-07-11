<script lang="ts">
	import { Avatar } from '@skeletonlabs/skeleton';
	import type { Person } from './Person';

	export let person: Person | null;
</script>

<div class="p-4">
	{#if person !== null}
		<div class="flex justify-center p-4">
			<Avatar src={person.avatar()} initials={person.initials()} width="w-60" />
		</div>
		<p class="font-bold text-center">{person.name()}</p>
		<section class="p-4">
			<div class="table-container">
				<table class="table table-hover">
					<tbody>
						{#each person.info as [key, value]}
							<tr>
								<td class="table-cell-fit">{key}</td>
								<td class="table-cell-fit">{value}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</section>
	{:else}
		<div class="flex justify-center p-4">
			<div class="placeholder-circle animate-pulse w-60" />
		</div>
		<p class="text-center">No person selected.</p>
	{/if}
</div>
