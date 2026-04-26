<template>
  <div class="p-4">
    <h2 class="text-sm font-semibold text-[#a0a0a0] uppercase tracking-wider mb-3">
      AI Assistant
    </h2>
    
    <div class="bg-[#121212] border border-[#333] p-3 mb-3 h-40 overflow-y-auto">
      <div v-if="messages.length === 0" class="text-[#666] text-sm h-full flex items-center justify-center">
        Describe the sound you want...
      </div>
      <div v-else class="space-y-2">
        <div 
          v-for="(msg, index) in messages" 
          :key="index"
          :class="[
            'text-sm p-2 border-l-2',
            msg.role === 'user' 
              ? 'border-amber-500 bg-amber-500/5 ml-4' 
              : 'border-[#444] bg-[#1e1e1e] mr-4'
          ]"
        >
          <div class="text-xs text-[#666] mb-1">{{ msg.role === 'user' ? 'You' : 'AI' }}</div>
          <div class="text-[#ccc]">{{ msg.content }}</div>
        </div>
      </div>
      <div v-if="thinking" class="text-[#666] text-sm italic mt-2">Processing...</div>
    </div>
    
    <div class="flex gap-2">
      <input 
        v-model="inputMessage"
        @keypress.enter="sendMessage"
        type="text" 
        placeholder="e.g., 'Add some reverb' or 'Give me a metal guitar tone'"
        class="flex-1 px-3 py-2 bg-[#121212] border border-[#333] text-white placeholder-[#666] focus:border-amber-500 focus:outline-none"
      >
      <button 
        @click="sendMessage"
        :disabled="thinking || !inputMessage"
        class="px-4 py-2 bg-amber-500 text-black font-medium hover:bg-amber-400 disabled:bg-[#333] disabled:text-[#666] disabled:cursor-not-allowed transition-colors"
      >
        Send
      </button>
      <button 
        @click="clearHistory"
        :disabled="thinking"
        class="px-3 py-2 border border-[#444] text-[#a0a0a0] hover:border-[#555] hover:text-white disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        title="Clear conversation"
      >
        Clear
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useAudioStore } from '../stores/audioStore'

const store = useAudioStore()
const inputMessage = ref('')
const thinking = ref(false)
const messages = ref<{ role: 'user' | 'ai'; content: string; commands?: any[] }[]>([])
const lastCommands = ref<any[]>([])

async function sendMessage() {
  if (!inputMessage.value || thinking.value) return
  
  const userMsg = inputMessage.value
  messages.value.push({ role: 'user', content: userMsg })
  inputMessage.value = ''
  thinking.value = true
  
  try {
    const response = await store.aiChat(userMsg)
    lastCommands.value = response.commands || []
    messages.value.push({ 
      role: 'ai', 
      content: lastCommands.value.length > 0 
        ? `Executed ${lastCommands.value.length} command(s)`
        : 'Done',
      commands: lastCommands.value 
    })
    
    for (const cmd of lastCommands.value) {
      await executeCommand(cmd)
    }
    
    await store.refreshStatus()
    
    if (store.selectedPlugin) {
      await store.selectPlugin(store.selectedPlugin)
    }
  } catch (error) {
    messages.value.push({ 
      role: 'ai', 
      content: `Error: ${error}` 
    })
  } finally {
    thinking.value = false
  }
}

async function executeCommand(cmd: any) {
  if (cmd.LoadPlugin) {
    await store.loadPlugin(cmd.LoadPlugin.uri)
  } else if (cmd.RemovePlugin) {
    const id = cmd.RemovePlugin.id === '@last' 
      ? store.lastLoadedId 
      : cmd.RemovePlugin.id
    if (id) await store.removePlugin(id)
  } else if (cmd.SetParameter) {
    const pluginId = cmd.SetParameter.plugin_id === '@last'
      ? store.lastLoadedId || ''
      : cmd.SetParameter.plugin_id
    await store.setParameter(
      pluginId,
      cmd.SetParameter.param_name,
      cmd.SetParameter.value
    )
  } else if (cmd.SetBypass) {
    if (cmd.SetBypass.bypass !== store.bypass) {
      await store.toggleBypass()
    }
  } else if (cmd.ClearChain) {
    // TODO: implement clear chain
  }
}

async function clearHistory() {
  messages.value = []
  lastCommands.value = []
  await store.clearHistory()
}
</script>
