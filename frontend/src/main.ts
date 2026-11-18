import { createApp } from 'vue';
import App from './App.vue';
import { installTooltips } from './tooltip';
import './style.css';
const app = createApp(App);
installTooltips(app);
app.mount('#app');
