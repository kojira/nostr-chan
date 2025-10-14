import { useState, useEffect } from 'react';
import {
  Container, Box, Typography, IconButton, Paper, Button, TextField,
  Slider, InputAdornment
} from '@mui/material';
import { ArrowBack, Save, Search, Percent } from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';

export const RagSettingsPage = () => {
  const navigate = useNavigate();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [threshold, setThreshold] = useState(0.9);

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      const response = await fetch('/api/settings/rag');
      if (response.ok) {
        const data = await response.json();
        setThreshold(data.similarity_threshold);
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
      const response = await fetch('/api/settings/rag', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          similarity_threshold: threshold,
        }),
      });

      if (response.ok) {
        alert('✅ 設定を保存しました');
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
          <Search sx={{ fontSize: 32 }} />
          <Typography variant="h4" fontWeight="bold">
            RAG検索設定
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

      {/* 類似度閾値 */}
      <Paper sx={{ p: 3, mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
          <Percent />
          <Typography variant="h6" fontWeight="bold">
            類似度閾値
          </Typography>
        </Box>
        <TextField
          type="number"
          value={threshold}
          onChange={(e) => {
            const val = parseFloat(e.target.value);
            if (!isNaN(val)) {
              setThreshold(Math.max(0, Math.min(1, val)));
            }
          }}
          fullWidth
          InputProps={{
            inputProps: { step: 0.01, min: 0, max: 1 },
            endAdornment: (
              <InputAdornment position="end">
                <Typography variant="caption" color="text.secondary">
                  ({(threshold * 100).toFixed(0)}%)
                </Typography>
              </InputAdornment>
            ),
          }}
          helperText="0.0〜1.0の範囲で設定（高いほど厳格）"
          sx={{ mb: 2 }}
        />
        <Slider
          value={threshold}
          onChange={(_, value) => setThreshold(value as number)}
          min={0.5}
          max={1.0}
          step={0.01}
          marks={[
            { value: 0.5, label: '0.5' },
            { value: 0.7, label: '0.7' },
            { value: 0.8, label: '0.8' },
            { value: 0.9, label: '0.9' },
            { value: 1.0, label: '1.0' },
          ]}
        />
        <Box sx={{ display: 'flex', gap: 1, mt: 2, flexWrap: 'wrap' }}>
          <Button variant="outlined" onClick={() => setThreshold(0.7)} size="small">
            緩め (0.7)
          </Button>
          <Button variant="outlined" onClick={() => setThreshold(0.8)} size="small">
            普通 (0.8)
          </Button>
          <Button variant="outlined" onClick={() => setThreshold(0.9)} size="small">
            厳格 (0.9)
          </Button>
          <Button variant="outlined" onClick={() => setThreshold(0.95)} size="small">
            最厳格 (0.95)
          </Button>
        </Box>
        <Paper sx={{ mt: 2, p: 2, bgcolor: 'grey.50' }}>
          <Typography variant="caption" color="text.secondary">
            💡 RAG検索（意味検索）で結果を返す最小類似度です。
            高いほど精度が高くなりますが、検索結果が少なくなります。
          </Typography>
        </Paper>
      </Paper>

      {/* 説明セクション */}
      <Paper sx={{ p: 3, bgcolor: 'info.light' }}>
        <Typography variant="subtitle1" fontWeight="bold" gutterBottom>
          RAG検索とは？
        </Typography>
        <Typography variant="body2" paragraph>
          Retrieval-Augmented Generation（検索拡張生成）の略で、
          過去の投稿内容を意味的に検索し、関連する情報を取得する機能です。
        </Typography>
        <Typography variant="subtitle2" fontWeight="bold" gutterBottom>
          類似度閾値の影響
        </Typography>
        <Typography variant="body2" component="div">
          <ul style={{ margin: 0, paddingLeft: 20 }}>
            <li><strong>低い値 (0.7-0.8)</strong>: より多くの結果が返されますが、関連性の低い結果も含まれる可能性があります</li>
            <li><strong>高い値 (0.9-0.95)</strong>: 関連性の高い結果のみが返されますが、検索結果が少なくなる可能性があります</li>
          </ul>
        </Typography>
      </Paper>
    </Container>
  );
};

