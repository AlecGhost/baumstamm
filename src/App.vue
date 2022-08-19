<script setup lang="ts">
import { ref } from 'vue'
import BaumstammGrid from './components/BaumstammGrid.vue'
import ConnectorType from './types/ConnectorType';
import { GridItemType, GridItemWrapper } from './types/GridItemWrapper';
import Person from './types/Person';

const rows = ref(1)
const columns = ref(1)

function zoomIn() {
  columns.value--
  rows.value--
}

function zoomOut() {
  columns.value++
  rows.value++
}

const gridItems = [new GridItemWrapper(GridItemType.Person, new Person('John', 'Doe', null, null))]

</script>

<template>
  <button @click="zoomOut">+</button>
  <button @click="zoomIn">-</button>
  <BaumstammGrid :columns="columns" :rows="rows" :gridItems="gridItems"/>
</template>

