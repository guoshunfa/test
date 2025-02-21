import { createApp } from "vue";
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import App from "./App.vue";
import 'element-plus/theme-chalk/el-upload.css'
import 'element-plus/theme-chalk/el-divider.css'

const app = createApp(App)
app.use(ElementPlus)
app.mount("#app");
