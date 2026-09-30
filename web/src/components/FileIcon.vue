<script setup lang="ts">
import { computed } from 'vue'
import { Folder, Image, Film, Music, FileText, FileCode, Archive, AppWindow, FileSpreadsheet, Presentation, Database, Type, Disc3, Download, KeyRound, FileCog, File } from '@lucide/vue'
import { fileCategory, typeLabels } from '../app/file-types'
const props = defineProps<{ name: string; kind?: string; mime?: string }>()
const category = computed(() => fileCategory(props.name,props.kind,props.mime))
const icons = { folder:Folder, image:Image, video:Film, audio:Music, pdf:FileText, text:FileText, code:FileCode, archive:Archive, binary:AppWindow, document:FileText, spreadsheet:FileSpreadsheet, presentation:Presentation, database:Database, font:Type, disk:Disc3, torrent:Download, certificate:KeyRound, config:FileCog, unknown:File }
</script>
<template><span class="file-icon" :data-category="category" :title="typeLabels[category]" aria-hidden="true"><component :is="icons[category]" :size="24" /></span></template>
