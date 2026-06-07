<script setup>
import { ref } from "vue";
import MainView from "./views/MainView.vue";
import SettingsView from "./views/SettingsView.vue";
import ModsView from "./views/ModsView.vue";
import ExtraView from "./views/ExtraView.vue";
import LauncherView from "./views/LauncherView.vue";
import AlternativeDownloadView from "./views/AlternativeDownloadView.vue";
import DownloadView from "./views/DownloadView.vue";
import DisableView from "./views/DisableView.vue";
import DuplicatesView from "./views/DuplicatesView.vue";
import JavaView from "./views/JavaView.vue";

const history = ref(['main']);

function currentView() {
  return history.value[history.value.length - 1];
}

function goTo(view) {
  history.value.push(view);
}

function goBack() {
  history.value.pop();
  if (history.value.length === 0) history.value = ['main'];
}
</script>

<template>
  <main class="app-container">
    <MainView v-if="currentView() === 'main'" @navigate="goTo" />
    <SettingsView v-else-if="currentView() === 'settings'" @back="goBack" />
    <ModsView v-else-if="currentView() === 'mody'" @navigate="goTo" @back="goBack" />
    <ExtraView v-else-if="currentView() === 'extra'" @back="goBack" />
    <LauncherView v-else-if="currentView() === 'launcher'" @navigate="goTo" @back="goBack" />
    <AlternativeDownloadView v-else-if="currentView() === 'alternativeDownload'" @back="goBack" />
    <DownloadView v-else-if="currentView() === 'download'" @back="goBack" />
    <DisableView v-else-if="currentView() === 'disable'" @back="goBack" />
    <DuplicatesView v-else-if="currentView() === 'duplicates'" @back="goBack" />
    <JavaView v-else-if="currentView() === 'java'" @back="goBack" />
  </main>
</template>

<style>
@import url('https://fonts.googleapis.com/css2?family=Pixelify+Sans:wght@400..700&family=Inter:wght@400;500;600&display=swap');

@import './styles/variables.css';

html {
  background-color: var(--bg-color);
  overscroll-behavior: none;
}

body {
  margin: 0;
  padding: 0;
  font-family: 'Inter', sans-serif;
  color: var(--text-color);
  background-color: var(--bg-color);
  background-image: url(data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAEAAAABABAMAAABYR2ztAAABgWlDQ1BJQ0MgUHJvZmlsZQAAKM+VkU8oBFEcxz8GkXZzsAfJYQ7LiRKSI0tJUduiLA5mZu1SO2Ob2c3FUbkqB38uFgcXZ64Orkopf0pubk7ERRq/N6t2U6u8er1P3/e+3977PtCKWcv26nrAdvJuYiymzybn9IZn6gkTQmPAsLzccDw+QdXxcUuNWm+6VRb/G+HUkmdBjS48ZOXcvPCi8MBaPqd4VzhiLRsp4VPhLlcuKHyvdLPEL4ozAWsqM+JOJ0aEI8J6poLNCraWXVu4Xziash3J12ZLnFK8rtjOFqyfe6oXhpacmSmly2xnjHEmiaNjUmCFLHm6ZXVE8UjIfqyKvy3wx8VlimsFSxyjrGJjBH7UH/zu1kv39ZaSQjGof/L9tw5o2IavLd//PPT9ryOofYQLp+xfLcLgu+hbZS16AM0bcHZZ1swdON+E1oec4RqBVCtTS6fh9US+KQkt19A0X+rtZ5/jO5iWriauYG8fOjOSvVDl3Y2Vvf15JuiP2DdDTnKUicDMbgAAAA9QTFRFMTExKSkpISEhGRkbExMWkZMtKQAAAAlwSFlzAAALEwAACxMBAJqcGAAABoRpVFh0WE1MOmNvbS5hZG9iZS54bXAAAAAAADw/eHBhY2tldCBiZWdpbj0i77u/IiBpZD0iVzVNME1wQ2VoaUh6cmVTek5UY3prYzlkIj8+DQo8eDp4bXBtZXRhIHhtbG5zOng9ImFkb2JlOm5zOm1ldGEvIiB4OnhtcHRrPSJYTVAgQ29yZSA0LjQuMC1FeGl2MiI+DQogIDxyZGY6UkRGIHhtbG5zOnJkZj0iaHR0cDovL3d3dy53My5vcmcvMTk5OS8wMi8yMi1yZGYtc3ludGF4LW5zIyI+DQogICAgPHJkZjpEZXNjcmlwdGlvbiByZGY6YWJvdXQ9IiIgeG1sbnM6eG1wTU09Imh0dHA6Ly9ucy5hZG9iZS5jb20veGFwLzEuMC9tbS8iIHhtbG5zOnN0RXZ0PSJodHRwOi8vbnMuYWRvYmUuY29tL3hhcC8xLjAvc1R5cGUvUmVzb3VyY2VFdmVudCMiIHhtbG5zOmRjPSJodHRwOi8vcHVybC5vcmcvZGMvZWxlbWVudHMvMS4xLyIgeG1sbnM6R0lNUD0iaHR0cDovL3d3dy5naW1wLm9yZy94bXAvIiB4bWxuczp0aWZmPSJodHRwOi8vbnMuYWRvYmUuY29tL3RpZmYvMS4wLyIgeG1sbnM6eG1wPSJodHRwOi8vbnMuYWRvYmUuY29tL3hhcC8xLjAvIiB4bXBNTTpEb2N1bWVudElEPSJnaW1wOmRvY2lkOmdpbXA6YTdmNmUwYTEtOGZhNi00ZDlmLTljYTItZjQ3ZGQ4NDIwMGUxIiB4bXBNTTpJbnN0YW5jZUlEPSJ4bXAuaWlkOjY4NzA3YzlhLWY5Y2YtNDRhOC04NzlhLTBmYWFhMGQ3NDFiZCIgeG1wTU06T3JpZ2luYWxEb2N1bWVudElEPSJ4bXAuZGlkOjI0MDgwOTY5LWY2ZDMtNDQzMy1iNGRiLWU0NDQwOTgzNDA3YSIgZGM6Rm9ybWF0PSJpbWFnZS9wbmciIEdJTVA6QVBJPSIyLjAiIEdJTVA6UGxhdGZvcm09IkxpbnV4IiBHSU1QOlRpbWVTdGFtcD0iMTYyMDkzMTMyMDk1MjgzNyIgR0lNUDpWZXJzaW9uPSIyLjEwLjI0IiB0aWZmOk9yaWVudGF0aW9uPSIxIiB4bXA6Q3JlYXRvclRvb2w9IkdJTVAgMi4xMCI+DQogICAgICA8eG1wTU06SGlzdG9yeT4NCiAgICAgICAgPHJkZjpTZXE+DQogICAgICAgICAgPHJkZjpsaSBzdEV2dDphY3Rpb249InNhdmVkIiBzdEV2dDpjaGFuZ2VkPSIvIiBzdEV2dDppbnN0YW5jZUlEPSJ4bXAuaWlkOmM4ZmYxYTM4LTJmYTctNDI0YS1hZDZkLWFmZWRkYzg5OTJjMiIgc3RFdnQ6c29mdHdhcmVBZ2VudD0iR2ltcCAyLjEwIChMaW51eCkiIHN0RXZ0OndoZW49IjIwMjEtMDUtMTFUMjE6NTc6NTIrMDE6MDAiIC8+DQogICAgICAgICAgPHJkZjpsaSBzdEV2dDphY3Rpb249InNhdmVkIiBzdEV2dDpjaGFuZ2VkPSIvIiBzdEV2dDppbnN0YW5jZUlEPSJ4bXAuaWlkOjQzNTZiZGJhLThhYTktNDdkYy04YTM3LTY1YTdmYzExMjg0NyIgc3RFdnQ6c29mdHdhcmVBZ2VudD0iR2ltcCAyLjEwIChMaW51eCkiIHN0RXZ0OndoZW49IjIwMjEtMDUtMTJUMjI6MjU6MDErMDE6MDAiIC8+DQogICAgICAgICAgPHJkZjpsaSBzdEV2dDphY3Rpb249InNhdmVkIiBzdEV2dDpjaGFuZ2VkPSIvIiBzdEV2dDppbnN0YW5jZUJEPSJ4bXAuaWlkOjUyZDM1ZWI4LTBlOGYtNDgyZS1iNzFkLTllZWVlMmQ0ZDFiNSIgc3RFdnQ6c29mdHdhcmVBZ2VudD0iR2ltcCAyLjEwIChMaW51eCkiIHN0RXZ0OndoZW49IjIwMjEtMDUtMTNUMTk6NDI6MDArMDE6MDAiIC8+DQogICAgICAgIDwvcmRmOlNlcT4NCiAgICAgIDwveG1wTU06SGlzdG9yeT4NCiAgICA8L3JkZjpEZXNjcmlwdGlvbj4NCiAgPC9yZGY6UkRGPg0KPC94OnhtcG1ldGE+DQo8P3hwYWNrZXQgZW5kPSJyIj8+aWV5wQAAAr9JREFUSMdtVVti5CAM8+xeAJIeYIFeIJALdIb7n2ltySaZtvloB1DwQ7IiIo+8VX/G6CXbs3YSALFqY1Scrx0AruX4CciSLIIe8AIPYRFat72C99v5Gv70WvycL1XB7zkXQm/NHk6vaACMc85ZfwL09+hiF+T88bRFYwyPMGyjS97HoYn6tt71jHyZmFjSIlHa5INg/Pc7AGcO2DQpDeG1aQgF6Wof+eMrG0Czit611QheqS81BQzbWecokwQgbJP3yglPkuRBwJDXW+WMkOWRhJycMg1xO0dGepxw3hQwyUo0mimLJ65VvAH0AiqKKQBwRghjYgGiM806udv2kgByKyHLMiRBMiz05QD7S9laz1S1AZhLUKFKB+iV9sLnHBGtIKLCKXu/0HRH+fcgpkTXKy8YrEhxTXVo+HwDQLgxH9ApAEsL0AehDXIBCTYXdqwb+v6hDVIBbYbgOQBY44FSClscFAiXAUgSHFyARfUYJYcMGkTfOXpaNRHnE2LcrqgdAIwYABU013KFDQBXk0WRiQsQCZw6dCWhaKMj3EDu50GzcYHZ1FaIn5sB1GsAaBpWqtyPHdAIGAR0Yp8ERHTE6J8vm+6tfj6XZ9WgARuqyL80wa3ejA5EdzicuEvakBr3vB5EGxxyE4jcWnv4ILj1dHdZn7EGrpUKumlDE5Pp1dkzru0F+cMiASiWwOL/wASIcdPdRhcA5BogPUxyTr8NxQMAkq/4R3wqLjMCIGzXATeqvwPkHTAcsIX9/YsQoQB8O2R3NZhc7oI9bY6O7HSD+/AGSj4sW/bD7piDgHF8AxxMbDcGtPCdbdbf5tpfu3UnLwhyebEiiIeOGbZcPRf0LNdlmOIWs7RAtw5AEokr2GHM/IPiL3eA2xhSTe4ZtHUPEYJXQEnLlMoCrImYkypwn4wqNp+Iau26vmC6mfN/fvYVIgCBUOYAAAAASUVORK5CYII=);
  background-attachment: fixed;
  background-repeat: repeat;
  min-height: 100vh;
  display: flex;
  justify-content: center;
  align-items: center;
  overscroll-behavior: none;
}

.app-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 100%;
  max-width: 800px;
  padding: 2rem;
  box-sizing: border-box;
}

.logo-container {
  margin-bottom: 1.5rem;
}

.big-o {
  width: 148px;
  height: 148px;
  object-fit: contain;
  filter: drop-shadow(4px 4px 0px #4a3621) drop-shadow(0px 8px 15px rgba(0,0,0,0.4));
}

.title {
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 2.5rem;
  font-weight: 600;
  margin: 0 0 1rem 0;
  text-align: center;
  letter-spacing: 1px;
}

.subtitle {
  font-size: 1.1rem;
  text-align: center;
  color: #d8c8b8;
  margin-bottom: 3rem;
  line-height: 1.5;
}

.buttons-grid {
  display: flex;
  justify-content: center;
  gap: 1.5rem;
  margin-bottom: 1.5rem;
  width: 100%;
  flex-wrap: wrap;
}

.button-card {
  background-color: var(--card-bg);
  border-radius: 16px;
  padding: 2rem 1rem;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  width: 140px;
  height: 140px;
  cursor: pointer;
  transition: all 0.2s ease;
  box-shadow: 0 4px 6px rgba(0,0,0,0.1);
}

.button-card:hover {
  background-color: var(--card-hover);
  transform: translateY(-2px);
  box-shadow: 0 6px 12px rgba(0,0,0,0.15);
}

.button-card:active {
  transform: translateY(0);
}

.icon {
  width: 48px;
  height: 48px;
  margin-bottom: 1rem;
  display: flex;
  align-items: center;
  justify-content: center;
}

.icon svg {
  width: 100%;
  height: 100%;
  color: #ffffff;
}

.button-label {
  font-weight: 600;
  font-size: 1.05rem;
  font-family: 'Pixelify Sans', sans-serif;
  letter-spacing: 0.5px;
}

.view {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 100%;
}

.view--full {
  flex: 1;
  width: 100%;
}

.back-row {
  width: 100%;
  display: flex;
  justify-content: flex-start;
  margin-bottom: 1rem;
}

.back-button {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;
  color: #d8c8b8;
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 1.1rem;
  transition: color 0.2s ease;
}

.back-button:hover {
  color: #ffffff;
}

.back-button svg {
  width: 24px;
  height: 24px;
}

.section-title {
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 2rem;
  font-weight: 600;
  margin: 0 0 2rem 0;
  text-align: center;
  letter-spacing: 1px;
  color: var(--text-color);
}

@media (max-width: 600px) {
  .buttons-grid {
    gap: 1rem;
  }
  .button-card {
    width: 120px;
    height: 120px;
    padding: 1.5rem 0.5rem;
  }
  .title {
    font-size: 2rem;
  }
}
</style>
