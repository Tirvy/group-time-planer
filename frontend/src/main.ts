import { createApp } from 'vue'

import '@fontsource/roboto/400.css'
import '@fontsource/roboto/500.css'
import '@fontsource/roboto/700.css'

import App from './App.vue'
import { vuetify } from './plugins/vuetify'
import './styles/main.css'

createApp(App).use(vuetify).mount('#app')
