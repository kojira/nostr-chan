import { useState, useEffect } from 'react';
import {
  Container, Box, Typography, IconButton, Paper, Button, TextField,
  List, ListItem, ListItemText, ListItemSecondaryAction, Chip
} from '@mui/material';
import { ArrowBack, Save, Wifi, Add, Delete } from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';

export const RelaySettingsPage = () => {
  const navigate = useNavigate();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [writeRelays, setWriteRelays] = useState<string[]>([]);
  const [readRelays, setReadRelays] = useState<string[]>([]);
  const [searchRelays, setSearchRelays] = useState<string[]>([]);
  const [newWriteRelay, setNewWriteRelay] = useState('');
  const [newReadRelay, setNewReadRelay] = useState('');
  const [newSearchRelay, setNewSearchRelay] = useState('');

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      const response = await fetch('/api/settings/relay');
      if (response.ok) {
        const data = await response.json();
        setWriteRelays(data.write || []);
        setReadRelays(data.read || []);
        setSearchRelays(data.search || []);
      }
    } catch (error) {
      console.error('設定読み込みエラー:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const response = await fetch('/api/settings/relay', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          write: writeRelays,
          read: readRelays,
          search: searchRelays,
        }),
      });

      if (response.ok) {
        alert('✅ 設定を保存しました\n⚠️ 変更を反映するにはBotの再起動が必要です');
      } else {
        alert('❌ 設定の保存に失敗しました');
      }
    } catch (error) {
      console.error('保存エラー:', error);
      alert('❌ 設定の保存に失敗しました');
    } finally {
      setSaving(false);
    }
  };

  const addRelay = (type: 'write' | 'read' | 'search', url: string) => {
    if (!url.startsWith('wss://') && !url.startsWith('ws://')) {
      alert('リレーURLは wss:// または ws:// で始まる必要があります');
      return;
    }

    if (type === 'write') {
      if (!writeRelays.includes(url)) {
        setWriteRelays([...writeRelays, url]);
        setNewWriteRelay('');
      }
    } else if (type === 'read') {
      if (!readRelays.includes(url)) {
        setReadRelays([...readRelays, url]);
        setNewReadRelay('');
      }
    } else if (type === 'search') {
      if (!searchRelays.includes(url)) {
        setSearchRelays([...searchRelays, url]);
        setNewSearchRelay('');
      }
    }
  };

  const removeRelay = (type: 'write' | 'read' | 'search', url: string) => {
    if (type === 'write') {
      setWriteRelays(writeRelays.filter(r => r !== url));
    } else if (type === 'read') {
      setReadRelays(readRelays.filter(r => r !== url));
    } else if (type === 'search') {
      setSearchRelays(searchRelays.filter(r => r !== url));
    }
  };

  if (loading) {
    return null;
  }

  return (
    <Container maxWidth="md" sx={{ py: 4 }}>
      {/* ヘッダー */}
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mb: 4 }}>
        <IconButton onClick={() => navigate('/')} size="large">
          <ArrowBack />
        </IconButton>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, flex: 1 }}>
          <Wifi sx={{ fontSize: 32 }} />
          <Typography variant="h4" fontWeight="bold">
            リレー設定
          </Typography>
        </Box>
        <Button
          variant="contained"
          startIcon={<Save />}
          onClick={handleSave}
          disabled={saving}
          size="large"
        >
          {saving ? '保存中...' : '保存'}
        </Button>
      </Box>

      {/* 書き込みリレー */}
      <Paper sx={{ p: 3, mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
          <Typography variant="h6" fontWeight="bold">
            書き込みリレー
          </Typography>
          <Chip label={`${writeRelays.length}個`} size="small" color="primary" />
        </Box>
        <List>
          {writeRelays.map((relay, index) => (
            <ListItem key={index} sx={{ bgcolor: 'grey.50', mb: 1, borderRadius: 1 }}>
              <ListItemText primary={relay} primaryTypographyProps={{ fontFamily: 'monospace' }} />
              <ListItemSecondaryAction>
                <IconButton edge="end" onClick={() => removeRelay('write', relay)} size="small">
                  <Delete />
                </IconButton>
              </ListItemSecondaryAction>
            </ListItem>
          ))}
        </List>
        <Box sx={{ display: 'flex', gap: 1, mt: 2 }}>
          <TextField
            fullWidth
            size="small"
            placeholder="wss://relay.example.com"
            value={newWriteRelay}
            onChange={(e) => setNewWriteRelay(e.target.value)}
            onKeyPress={(e) => e.key === 'Enter' && addRelay('write', newWriteRelay)}
          />
          <Button
            variant="outlined"
            startIcon={<Add />}
            onClick={() => addRelay('write', newWriteRelay)}
            disabled={!newWriteRelay}
          >
            追加
          </Button>
        </Box>
      </Paper>

      {/* 読み込みリレー */}
      <Paper sx={{ p: 3, mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
          <Typography variant="h6" fontWeight="bold">
            読み込みリレー
          </Typography>
          <Chip label={`${readRelays.length}個`} size="small" color="success" />
        </Box>
        <List>
          {readRelays.map((relay, index) => (
            <ListItem key={index} sx={{ bgcolor: 'grey.50', mb: 1, borderRadius: 1 }}>
              <ListItemText primary={relay} primaryTypographyProps={{ fontFamily: 'monospace' }} />
              <ListItemSecondaryAction>
                <IconButton edge="end" onClick={() => removeRelay('read', relay)} size="small">
                  <Delete />
                </IconButton>
              </ListItemSecondaryAction>
            </ListItem>
          ))}
        </List>
        <Box sx={{ display: 'flex', gap: 1, mt: 2 }}>
          <TextField
            fullWidth
            size="small"
            placeholder="wss://relay.example.com"
            value={newReadRelay}
            onChange={(e) => setNewReadRelay(e.target.value)}
            onKeyPress={(e) => e.key === 'Enter' && addRelay('read', newReadRelay)}
          />
          <Button
            variant="outlined"
            startIcon={<Add />}
            onClick={() => addRelay('read', newReadRelay)}
            disabled={!newReadRelay}
          >
            追加
          </Button>
        </Box>
      </Paper>

      {/* 検索リレー */}
      <Paper sx={{ p: 3, mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
          <Typography variant="h6" fontWeight="bold">
            検索リレー
          </Typography>
          <Chip label={`${searchRelays.length}個`} size="small" color="secondary" />
        </Box>
        <List>
          {searchRelays.map((relay, index) => (
            <ListItem key={index} sx={{ bgcolor: 'grey.50', mb: 1, borderRadius: 1 }}>
              <ListItemText primary={relay} primaryTypographyProps={{ fontFamily: 'monospace' }} />
              <ListItemSecondaryAction>
                <IconButton edge="end" onClick={() => removeRelay('search', relay)} size="small">
                  <Delete />
                </IconButton>
              </ListItemSecondaryAction>
            </ListItem>
          ))}
        </List>
        <Box sx={{ display: 'flex', gap: 1, mt: 2 }}>
          <TextField
            fullWidth
            size="small"
            placeholder="wss://search.relay.example.com"
            value={newSearchRelay}
            onChange={(e) => setNewSearchRelay(e.target.value)}
            onKeyPress={(e) => e.key === 'Enter' && addRelay('search', newSearchRelay)}
          />
          <Button
            variant="outlined"
            startIcon={<Add />}
            onClick={() => addRelay('search', newSearchRelay)}
            disabled={!newSearchRelay}
          >
            追加
          </Button>
        </Box>
      </Paper>

      {/* 注意事項 */}
      <Paper sx={{ p: 3, bgcolor: 'warning.light' }}>
        <Typography variant="subtitle1" fontWeight="bold" gutterBottom>
          ⚠️ 重要な注意事項
        </Typography>
        <Typography variant="body2">
          リレー設定を変更した後は、Botを再起動する必要があります。
          再起動しない限り、変更は反映されません。
        </Typography>
      </Paper>
    </Container>
  );
};

