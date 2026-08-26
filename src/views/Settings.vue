<script setup lang="ts">
// ============================================================
// 理论训练考核系统 — 设置页（首选项 + 功能入口 + 关于）
// ============================================================
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  SettingOutlined,
  FolderOutlined,
  FileTextOutlined,
  DatabaseOutlined,
  QuestionCircleOutlined,
  RightOutlined,
  InfoCircleOutlined,
  ReadOutlined,
} from '@ant-design/icons-vue'
import { usePreferencesStore } from '../stores/preferences'
import { APP_VERSION, APP_UPDATED_AT, CHANGELOG } from '../version'

const router = useRouter()
const prefs = usePreferencesStore()
const changelogOpen = ref(false)

const entries = [
  {
    key: 'banks',
    title: '题库管理',
    desc: '创建、重命名、删除题库',
    icon: FolderOutlined,
    path: '/banks',
  },
  {
    key: 'questions',
    title: '题目管理',
    desc: '进入题库后管理试题（导入 / 编辑）',
    icon: FileTextOutlined,
    path: '/banks',
  },
  {
    key: 'data',
    title: '数据管理',
    desc: '数据库导入、导出与合并',
    icon: DatabaseOutlined,
    path: '/data',
  },
  {
    key: 'guide',
    title: '使用说明',
    desc: '功能介绍与操作指引',
    icon: QuestionCircleOutlined,
    path: '/guide',
  },
]
</script>

<template>
  <div class="settings-page">
    <h2 style="margin-bottom: 24px">
      <SettingOutlined style="margin-right: 8px" />设置
    </h2>

    <!-- 首选项 -->
    <a-card title="首选项" size="small" style="margin-bottom: 16px">
      <p style="color: #888; margin-bottom: 16px; font-size: 13px">
        以下开关作为练习 / 模拟考试进入时的默认值，当场切换不会改写此处设置。
      </p>
      <a-form layout="horizontal" :label-col="{ span: 6 }" :wrapper-col="{ span: 18 }">
        <a-form-item label="不定项">
          <a-switch v-model:checked="prefs.indeterminateMode" />
          <span class="pref-hint">选择题不区分单选/多选，可自由选择一项或多项</span>
        </a-form-item>
        <a-form-item label="打乱答案顺序">
          <a-switch v-model:checked="prefs.sortAnswerOrder" />
          <span class="pref-hint">多选答案不区分顺序（A,B 与 B,A 都判正确）</span>
        </a-form-item>
        <a-form-item label="打乱选项顺序" style="margin-bottom: 0">
          <a-switch v-model:checked="prefs.shuffleOptions" />
          <span class="pref-hint">每道题的选项顺序随机打乱，防止背答案</span>
        </a-form-item>
      </a-form>
    </a-card>

    <!-- 功能入口 -->
    <a-card title="功能入口" size="small" style="margin-bottom: 16px">
      <a-list :data-source="entries" item-layout="horizontal">
        <template #renderItem="{ item }">
          <a-list-item class="settings-entry" @click="router.push(item.path)">
            <a-list-item-meta>
              <template #avatar>
                <component :is="item.icon" style="font-size: 20px; color: #1890ff" />
              </template>
              <template #title>{{ item.title }}</template>
              <template #description>{{ item.desc }}</template>
            </a-list-item-meta>
            <template #actions>
              <RightOutlined style="color: #bbb" />
            </template>
          </a-list-item>
        </template>
      </a-list>
    </a-card>

    <!-- 关于 -->
    <a-card size="small">
      <template #title>
        <InfoCircleOutlined style="margin-right: 6px" />关于
      </template>
      <a-descriptions :column="1" size="small" bordered style="margin-bottom: 16px">
        <a-descriptions-item label="版本号">{{ APP_VERSION }}</a-descriptions-item>
        <a-descriptions-item label="更新时间">{{ APP_UPDATED_AT }}</a-descriptions-item>
      </a-descriptions>
      <a-button @click="changelogOpen = true">
        <ReadOutlined /> 更新说明
      </a-button>
    </a-card>

    <a-modal
      v-model:open="changelogOpen"
      title="更新说明"
      :footer="null"
      width="560px"
    >
      <div v-for="entry in CHANGELOG" :key="entry.version" class="changelog-entry">
        <div class="changelog-header">
          <strong>v{{ entry.version }}</strong>
          <span class="changelog-date">{{ entry.date }}</span>
        </div>
        <ul>
          <li v-for="(item, i) in entry.items" :key="i">{{ item }}</li>
        </ul>
      </div>
    </a-modal>
  </div>
</template>

<style scoped>
.settings-page {
  max-width: 720px;
  margin: 0 auto;
}

.pref-hint {
  margin-left: 12px;
  color: #999;
  font-size: 12px;
}

.settings-entry {
  cursor: pointer;
  transition: background 0.15s;
}

.settings-entry:hover {
  background: #f5f5f5;
}

.changelog-entry + .changelog-entry {
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid #f0f0f0;
}

.changelog-header {
  display: flex;
  align-items: baseline;
  gap: 12px;
  margin-bottom: 8px;
}

.changelog-date {
  color: #999;
  font-size: 13px;
}

.changelog-entry ul {
  margin: 0;
  padding-left: 20px;
  line-height: 1.8;
  color: #333;
}
</style>
