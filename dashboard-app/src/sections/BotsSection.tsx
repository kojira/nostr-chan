import { useState } from 'react';
import { Box, Typography, Paper, Button, ToggleButtonGroup, ToggleButton } from '@mui/material';
import { SmartToy, Add, CheckCircle, Cancel, List } from '@mui/icons-material';
import { BotCard } from '../components/BotCard';
import { BotDialog } from '../components/BotDialog';
import type { BotData, BotRequest } from '../types';
import { botApi } from '../api/botApi';

interface BotsSectionProps {
  bots: BotData[];
  onRefresh: () => void;
}

export const BotsSection = ({ bots, onRefresh }: BotsSectionProps) => {
  const [filter, setFilter] = useState<'all' | 'active' | 'inactive'>('all');
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingBot, setEditingBot] = useState<BotData | null>(null);

  const filteredBots = bots.filter(bot => {
    if (filter === 'active') return bot.status === 0;
    if (filter === 'inactive') return bot.status === 1;
    return true;
  });

  const activeCount = bots.filter(b => b.status === 0).length;
  const inactiveCount = bots.filter(b => b.status === 1).length;

  const handleEdit = (bot: BotData) => {
    setEditingBot(bot);
    setDialogOpen(true);
  };

  const handleAdd = () => {
    setEditingBot(null);
    setDialogOpen(true);
  };

  const handleSave = async (data: BotRequest, pubkey?: string) => {
    try {
      if (pubkey) {
        await botApi.updateBot(pubkey, data);
      } else {
        await botApi.createBot(data);
      }
      setDialogOpen(false);
      onRefresh();
    } catch (error) {
      console.error('Bot保存エラー:', error);
      alert('❌ Bot保存に失敗しました');
    }
  };

  const handleDelete = async (pubkey: string) => {
    if (!confirm('このBotを削除しますか？')) return;
    try {
      await botApi.deleteBot(pubkey);
      onRefresh();
    } catch (error) {
      console.error('Bot削除エラー:', error);
      alert('❌ Bot削除に失敗しました');
    }
  };

  const handleToggle = async (pubkey: string) => {
    try {
      await botApi.toggleBot(pubkey);
      onRefresh();
    } catch (error) {
      console.error('Bot切替エラー:', error);
      alert('❌ Bot切替に失敗しました');
    }
  };

  return (
    <Paper elevation={0} sx={{ p: 3, mb: 3, border: '1px solid', borderColor: 'divider', borderRadius: 2 }}>
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 3 }}>
        <Typography variant="h5" fontWeight="bold" sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <SmartToy /> Bot管理
        </Typography>
        <Box sx={{ display: 'flex', gap: 2, alignItems: 'center' }}>
          <ToggleButtonGroup
            value={filter}
            exclusive
            onChange={(_, value) => value && setFilter(value)}
            size="small"
          >
            <ToggleButton value="all">
              <List sx={{ mr: 0.5 }} /> 全て ({bots.length})
            </ToggleButton>
            <ToggleButton value="active">
              <CheckCircle sx={{ mr: 0.5 }} /> 有効 ({activeCount})
            </ToggleButton>
            <ToggleButton value="inactive">
              <Cancel sx={{ mr: 0.5 }} /> 無効 ({inactiveCount})
            </ToggleButton>
          </ToggleButtonGroup>
          <Button
            variant="contained"
            startIcon={<Add />}
            onClick={handleAdd}
            sx={{
              background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
              '&:hover': {
                background: 'linear-gradient(135deg, #5568d3 0%, #6a3f8f 100%)',
              },
            }}
          >
            BOT追加
          </Button>
        </Box>
      </Box>

      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
        {filteredBots.map((bot) => (
          <BotCard
            key={bot.pubkey}
            bot={bot}
            onEdit={handleEdit}
            onDelete={handleDelete}
            onToggle={handleToggle}
          />
        ))}
        {filteredBots.length === 0 && (
          <Box sx={{ textAlign: 'center', py: 4, color: 'text.secondary' }}>
            {filter === 'all' ? 'Botが登録されていません' : `${filter === 'active' ? '有効な' : '無効な'}Botはありません`}
          </Box>
        )}
      </Box>

      <BotDialog
        open={dialogOpen}
        bot={editingBot}
        onClose={() => setDialogOpen(false)}
        onSave={handleSave}
      />
    </Paper>
  );
};

