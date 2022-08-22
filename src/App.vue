<script setup lang="ts">
import { onMounted, ref } from 'vue'
import BaumstammGrid from './components/BaumstammGrid.vue'
import { GridItemWrapper } from './types/GridItemWrapper'
import { invoke } from '@tauri-apps/api'

const rows = ref(1)
const columns = ref(3)
const gridItems = ref<GridItemWrapper[]>([])

onMounted(async () => {
  await updateGrid()
})


async function updateGrid() {
  gridItems.value = await generateGrid()
}

async function generateGrid(): Promise<GridItemWrapper[]> {
  return invoke('generate_grid', {
    'size': {
      'rows': rows.value,
      'columns': columns.value
    },
    'source': {
      'id': 0,
      'point': {
        'x': 0,
        'y': 0
      }
    }
  }).then((response) => {
    try {
      const itemArray = response as Array<any>;
      return itemArray.map(item => GridItemWrapper.deserialize(item))
    } catch (err) {
      throw err
    }
  }).catch((err) => {
    throw err
  })
}

async function zoomIn() {
  columns.value--
  rows.value--
  await updateGrid()
}

async function zoomOut() {
  columns.value++
  rows.value++
  await updateGrid()
}
</script>

<template>
  <button @click="zoomOut">+</button>
  <button @click="zoomIn">-</button>
  <BaumstammGrid :columns="columns" :rows="rows" :gridItems="gridItems" />
</template>

