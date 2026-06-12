<script setup lang="ts">
// ============================================================
// 刷题助手 — 根布局（侧边导航 + 内容区）
// ============================================================
import { h, computed, ref } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import zhCN from 'ant-design-vue/es/locale/zh_CN'
import { Modal } from 'ant-design-vue'
import {
  DashboardOutlined,
  FolderOutlined,
  BookOutlined,
  ThunderboltOutlined,
  BugOutlined,
  FormOutlined,
  QuestionCircleOutlined,
  ExclamationCircleOutlined,
  ClockCircleOutlined,
  DatabaseOutlined,
} from '@ant-design/icons-vue'
import type { MenuProps } from 'ant-design-vue'
import { usePracticeStore } from './stores/practice'

const router = useRouter()
const route = useRoute()
const collapsed = ref(false)

const menuItems: MenuProps['items'] = [
  { key: '/', icon: () => h(DashboardOutlined), label: '数据看板' },
  { key: '/banks', icon: () => h(FolderOutlined), label: '题库管理' },
  { key: '/data', icon: () => h(DatabaseOutlined), label: '数据管理' },
  {
    key: 'practice-group',
    label: '练习模式',
    type: 'group',
    children: [
      { key: '/sequential', icon: () => h(BookOutlined), label: '顺序练习' },
      { key: '/random', icon: () => h(ThunderboltOutlined), label: '随机练习' },
      { key: '/wrong', icon: () => h(BugOutlined), label: '错题练习' },
      { key: '/exam', icon: () => h(FormOutlined), label: '模拟考试' },
    ],
  },
  {
    key: '/memory',
    icon: () => h(ClockCircleOutlined),
    label: '练习记忆',
  },
  {
    key: '/guide',
    icon: () => h(QuestionCircleOutlined),
    label: '使用说明',
  },
]

const selectedKeys = computed(() => [route.path])

function onMenuClick({ key }: { key: string }) {
  if (!key || !key.startsWith('/') || key === route.path) return

  const practiceStore = usePracticeStore()
  const isOnPracticePage = route.path.startsWith('/practice/') || route.path.startsWith('/exam/')

  // 有进行中的练习/考试，且不是点到同一页面 → 弹出确认对话框
  if (isOnPracticePage && practiceStore.answeredCount > 0) {
    const isExam = route.path.startsWith('/exam/')
    Modal.confirm({
      title: isExam ? '退出考试？' : '退出练习？',
      icon: h(ExclamationCircleOutlined),
      content: `已作答 ${practiceStore.answeredCount} / ${practiceStore.totalCount} 题，退出后已保存的记录不会丢失。`,
      okText: '退出',
      cancelText: '继续答题',
      onOk: () => {
        practiceStore.skipLeaveConfirm = true
        router.push(key)
      },
    })
  } else {
    router.push(key)
  }
}
</script>

<template>
  <a-config-provider :locale="zhCN">
  <a-layout style="min-height: 100vh">
    <a-layout-sider
      v-model:collapsed="collapsed"
      collapsible
      :style="{ overflow: 'auto', height: '100vh', position: 'fixed', left: 0, top: 0, bottom: 0 }"
    >
      <div class="logo">
        <span v-if="!collapsed">
          <svg viewBox="0 0 24 24" width="22" height="22" fill="currentColor" style="margin-right: 8px; vertical-align: middle">
            <path d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04a1 1 0 000-1.41l-2.34-2.34a1 1 0 00-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"/>
          </svg>
          刷题助手
        </span>
        <span v-else>
          <svg viewBox="0 0 24 24" width="22" height="22" fill="currentColor" style="vertical-align: middle">
            <path d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04a1 1 0 000-1.41l-2.34-2.34a1 1 0 00-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"/>
          </svg>
        </span>
      </div>
      <a-menu
        theme="dark"
        mode="inline"
        :selected-keys="selectedKeys"
        :items="menuItems"
        @click="onMenuClick"
      />
    </a-layout-sider>

    <a-layout :style="{ marginLeft: collapsed ? '80px' : '200px', transition: 'margin-left 0.2s', minWidth: 0 }">
      <a-layout-header
        :style="{
          background: '#fff',
          padding: '0 24px',
          borderBottom: '1px solid #f0f0f0',
        }"
      >
        <h2 style="margin: 0; line-height: 64px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis">刷题助手</h2>
      </a-layout-header>
      <a-layout-content
        :style="{
          margin: '16px',
          padding: '16px',
          background: '#fff',
          borderRadius: '8px',
          minHeight: '280px',
          overflowX: 'auto',
        }"
      >
        <router-view />
      </a-layout-content>
    </a-layout>
  </a-layout>
  </a-config-provider>
</template>

<style>
/* 全局：麒麟 V10 等 Linux 平台按钮/输入框高度适配 */
.ant-btn,
.ant-btn:not(.ant-btn-sm) {
  min-height: 32px;
  line-height: 1.5;
  display: inline-flex;
  align-items: center;
}
.ant-btn-sm {
  min-height: 26px;
  line-height: 1.4;
}
/* 弹窗关闭按钮不受全局按钮样式影响 */
.ant-modal-close {
  min-height: auto !important;
  line-height: 1 !important;
  display: block !important;
  width: 54px;
  height: 54px;
}

.ant-input,
.ant-input-affix-wrapper {
  min-height: 32px;
}

.ant-input-number {
  min-height: 32px;
}
.ant-input-number-input {
  min-height: 30px !important;
  line-height: 30px !important;
}

.ant-select-single:not(.ant-select-customize-input) .ant-select-selector {
  min-height: 32px !important;
  height: auto !important;
}
.ant-select-single .ant-select-selection-item {
  line-height: 30px !important;
}

.ant-picker {
  min-height: 32px;
}

.ant-btn > span {
  display: inline-flex;
  align-items: center;
  white-space: nowrap;
}
.ant-btn-sm.ant-btn-icon-only {
  /* 纯图标小按钮不受影响 */;
}
/* 带文字的小按钮取消固定高度限制 */
.ant-btn-sm:not(.ant-btn-icon-only) {
  height: auto;
  min-height: 24px;
  padding-top: 2px;
  padding-bottom: 2px;
}

.ant-radio-button-wrapper {
  min-height: 32px;
  height: auto !important;
  line-height: 30px !important;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.logo {
  height: 64px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 18px;
  font-weight: bold;
  white-space: nowrap;
}
</style>
