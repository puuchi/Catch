import {onMounted,onBeforeUnmount,nextTick,watch,type Ref} from 'vue';import {ui} from './store';
export function useScroll(element:Ref<HTMLElement|undefined>,key:()=>string){
  const save=()=>{if(element.value)ui.scroll[key()]=element.value.scrollTop};
  const restore=async()=>{await nextTick();if(element.value)element.value.scrollTop=ui.scroll[key()]||0};
  onMounted(restore);watch(key,restore);onBeforeUnmount(save);return save;
}
