<template>
  <div class="bg-gray-800 rounded-lg p-4">
    <h2 class="text-xl font-bold mb-4 text-pink-400">AI Assistant</h2>
    
    <div class="bg-gray-900 rounded-lg p-4 mb-4 h-48 overflow-y-auto space-y-2">
      <div 
        v-for="(msg, index) in messages" 
        :key="index"
        :class="[
          'p-2 rounded',
          msg.role === 'user' ? 'bg-blue-900 ml-8' : 'bg-gray-700 mr-8'
        ]"
      >
        <div class="text-xs text-gray-400 mb-1">{{ msg.role === 'user' ? 'You' : 'AI' }}</div>
        <div>{{ msg.content }}</div>
        <div v-if="msg.commands" class="mt-2 text-xs text-green-400">
          Commands: {{ msg.commands.length }}
        </div>
      </div>
      <div v-if="thinking" class="text-gray-400 italic">Thinking...</div>
    </div>
    
    <div class="flex space-x-2">
      <input 
        v-model="inputMessage"
        @keypress.enter="sendMessage"
        type="text" 
        placeholder="Describe what you want (e.g., 'Add some reverb')..."
        class="flex-1 px-3 py-2 bg-gray-700 rounded text-white placeholder-gray-400"
      >
      <button 
        @click="sendMessage"
        :disabled="thinking || !inputMessage"
        class="px-4 py-2 bg-pink-500 hover:bg-pink-600 disabled:bg-gray-600 rounded"
      >
        Send
      </button>
      <button 
        @click="clearHistory"
        :disabled="thinking"
        class="px-4 py-2 bg-gray-600 hover:bg-gray-500 disabled:bg-gray-700 rounded"
        title="Clear conversation history"
      >
        Clear
      </button>
    </div>
    
    <div v-if="lastCommands.length > 0" class="mt-4">
      <h3 class="text-sm font-semibold mb-2">Last AI Commands:</h3>
      <div class="space-y-1 text-sm">
        <div 
          v-for="(cmd, idx) in lastCommands" 
          :key="idx"
          class="p-2 bg-gray-700 rounded"
        >
          {{ idx + 1 }}. {{ JSON.stringify(cmd) }}
        </div>
      </div>
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
      content: `Executed ${lastCommands.value.length} command(s)`,
      commands: lastCommands.value 
    })
    
    // Execute the commands
    for (const cmd of lastCommands.value) {
      await executeCommand(cmd)
    }
    
    await store.refreshStatus()
    
    // Refresh parameters if a plugin is selected
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