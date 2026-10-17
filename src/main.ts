import { mount } from "svelte";
import "@fontsource-variable/instrument-sans/wdth.css";
import "./app.css";
import App from "./App.svelte";

mount(App, { target: document.getElementById("app")! });
