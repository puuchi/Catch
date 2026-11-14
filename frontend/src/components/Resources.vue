<script setup lang="ts">
import {computed,ref} from 'vue';import Icon from './Icon.vue';import {host} from '../bridge';import {busy,dispatch,generation,inform,ui} from '../store';import type {Owner,Resource,ResourceKind} from '../models';
const props=withDefaults(defineProps<{owner:Owner;resources?:Resource[]}>(),{resources:()=>[]});const key=computed(()=>`${props.owner.kind}:${props.owner.id}`),expanded=computed({get:()=>ui.expandedResources?.includes(key.value)||false,set:value=>{ui.expandedResources=value?[...(ui.expandedResources||[]).filter(id=>id!==key.value),key.value]:(ui.expandedResources||[]).filter(id=>id!==key.value)}}),adding=ref(false),menu=ref<string|null>(null);
async function pick(kind:ResourceKind,resource?:Resource){
  adding.value=false;
  if(kind==='web'){ui.resourceDraft={owner:props.owner,kind,mode:resource?'relocate':'attach',id:resource?.id,target:resource?.target||'',title:resource?.title||''};return}
  try{const paths=await host.pick(generation.value,kind);for(const target of paths){await dispatch('resources',resource?'relocate':'attach',resource?{owner:props.owner,resourceId:resource.id,target}:{owner:props.owner,kind,target})}if(paths.length)expanded.value=true}catch(error){inform(error)}
}
async function open(resource:Resource){try{await host.open(generation.value,props.owner,resource.id)}catch(error){inform(error);menu.value=resource.id}}
async function remove(resource:Resource){try{await dispatch('resources','remove',{owner:props.owner,resourceId:resource.id});menu.value=null}catch(error){inform(error)}}
</script>
<template><div class="resources" :data-resource-kind="owner.kind" :data-resource-id="owner.id">
  <div class="resource-heading"><button class="quiet-button" :aria-expanded="expanded" @click="expanded=!expanded"><Icon name="clip"/>{{resources.length?`资料 ${resources.length}`:'资料'}}</button><button class="icon-button resource-add" title="添加资料" aria-label="添加资料" :disabled="!!busy" @click="adding=!adding"><Icon name="plus"/></button></div>
  <div v-if="adding" class="resource-options"><button @click="pick('file')">文件</button><button @click="pick('folder')">文件夹</button><button @click="pick('web')">网页</button></div>
  <div v-if="expanded&&resources.length" class="resource-list"><div v-for="resource in resources" :key="resource.id" class="resource-row"><button class="resource-open" :title="resource.target" @click="open(resource)"><Icon :name="resource.kind==='web'?'link':resource.kind"/><span>{{resource.title}}</span></button><button class="icon-button" :aria-label="`${resource.title}的操作`" @click="menu=menu===resource.id?null:resource.id">⋯</button><div v-if="menu===resource.id" class="resource-actions"><button @click="open(resource)">打开</button><button @click="pick(resource.kind,resource)">{{resource.kind==='web'?'修改地址':'重新定位'}}</button><button @click="remove(resource)">移除关联</button></div></div></div>
</div></template>
