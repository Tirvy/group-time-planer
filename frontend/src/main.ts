import { createApp } from 'vue'

// Only the Cyrillic and Latin subsets are needed: the UI is Russian and Latin
// characters appear in timezone identifiers and Latin display names.
import '@fontsource/roboto/cyrillic-400.css'
import '@fontsource/roboto/cyrillic-500.css'
import '@fontsource/roboto/cyrillic-700.css'
import '@fontsource/roboto/latin-400.css'
import '@fontsource/roboto/latin-500.css'
import '@fontsource/roboto/latin-700.css'

import App from './App.vue'
import { vuetify } from './plugins/vuetify'
import './styles/main.css'

createApp(App).use(vuetify).mount('#app')
