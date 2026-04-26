import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import './style.css'
import { useAudioStore } from './stores/audioStore'

const app = createApp(App)
const pinia = createPinia()
app.use(pinia)

// Initialize the audio store and set up event listeners
const audioStore = useAudioStore(pinia)
audioStore.setupParameterListener().catch(console.error)

app.mount('#app')
