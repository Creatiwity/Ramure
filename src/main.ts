import { createApp } from "vue";
import App from "./App.vue";
import { i18n } from "./i18n";
import "./styles/tokens.css";

document.documentElement.lang = i18n.global.locale.value;
createApp(App).use(i18n).mount("#app");
