import { useState, useEffect } from 'react';
import {
  Container, Box, Typography, IconButton, Paper, Button, TextField,
  Slider, InputAdornment
} from '@mui/material';
import { ArrowBack, Save, Psychology, Timer, TextFields } from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';

export const GptSettingsPage = () => {
  const navigate = useNavigate();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [answerLength, setAnswerLength] = useState(100);
  const [timeout, setTimeout] = useState(60);
  const [geminiSearchTimeout, setGeminiSearchTimeout] = useState(180);
  const [recentContextCount, setRecentContextCount] = useState(10);
  const [summaryThreshold, setSummaryThreshold] = useState(5000);
  const [maxSummaryTokens, setMaxSummaryTokens] = useState(8000);
  const [maxImpressionLength, setMaxImpressionLength] = useState(500);
  const [maxMentalDiaryLength, setMaxMentalDiaryLength] = useState(1000);

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      const response = await fetch('/api/settings/gpt');
      if (response.ok) {
        const data = await response.json();
        setAnswerLength(data.answer_length);
        setTimeout(data.timeout);
        setGeminiSearchTimeout(data.gemini_search_timeout || 180);
        setRecentContextCount(data.recent_context_count || 10);
        setSummaryThreshold(data.summary_threshold || 5000);
        setMaxSummaryTokens(data.max_summary_tokens || 8000);
        setMaxImpressionLength(data.max_impression_length || 500);
        setMaxMentalDiaryLength(data.max_mental_diary_length || 1000);
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
      const response = await fetch('/api/settings/gpt', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          answer_length: answerLength,
          timeout: timeout,
          gemini_search_timeout: geminiSearchTimeout,
          recent_context_count: recentContextCount,
          summary_threshold: summaryThreshold,
          max_summary_tokens: maxSummaryTokens,
          max_impression_length: maxImpressionLength,
          max_mental_diary_length: maxMentalDiaryLength,
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
          <Psychology sx={{ fontSize: 32 }} />
          <Typography variant="h4" fontWeight="bold">
            GPT設定
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

      {/* 回答長 */}
      <Paper sx={{ p: 3, mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
          <TextFields />
          <Typography variant="h6" fontWeight="bold">
            回答長
          </Typography>
        </Box>
        <TextField
          type="number"
          value={answerLength}
          onChange={(e) => setAnswerLength(Math.max(10, Math.min(1000, parseInt(e.target.value) || 10)))}
          fullWidth
          InputProps={{
            endAdornment: <InputAdornment position="end">文字</InputAdornment>,
          }}
          helperText="10〜1000文字の範囲で設定"
          sx={{ mb: 2 }}
        />
        <Slider
          value={answerLength}
          onChange={(_, value) => setAnswerLength(value as number)}
          min={10}
          max={1000}
          step={10}
          marks={[
            { value: 10, label: '10' },
            { value: 250, label: '250' },
            { value: 500, label: '500' },
            { value: 750, label: '750' },
            { value: 1000, label: '1000' },
          ]}
        />
        <Box sx={{ display: 'flex', gap: 1, mt: 2, flexWrap: 'wrap' }}>
          <Button variant="outlined" onClick={() => setAnswerLength(50)} size="small">
            短め (50文字)
          </Button>
          <Button variant="outlined" onClick={() => setAnswerLength(100)} size="small">
            普通 (100文字)
          </Button>
          <Button variant="outlined" onClick={() => setAnswerLength(200)} size="small">
            長め (200文字)
          </Button>
          <Button variant="outlined" onClick={() => setAnswerLength(500)} size="small">
            詳細 (500文字)
          </Button>
          <Button variant="outlined" onClick={() => setAnswerLength(1000)} size="small">
            最大 (1000文字)
          </Button>
        </Box>
        <Paper sx={{ mt: 2, p: 2, bgcolor: 'grey.50' }}>
          <Typography variant="caption" color="text.secondary">
            💡 Botが生成する返信の目安となる文字数です。実際の返信はこれより多少前後します。
          </Typography>
        </Paper>
      </Paper>

      {/* タイムアウト */}
      <Paper sx={{ p: 3, mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
          <Timer />
          <Typography variant="h6" fontWeight="bold">
            GPTタイムアウト
          </Typography>
        </Box>
        <TextField
          type="number"
          value={timeout}
          onChange={(e) => setTimeout(Math.max(10, Math.min(300, parseInt(e.target.value) || 10)))}
          fullWidth
          InputProps={{
            endAdornment: <InputAdornment position="end">秒</InputAdornment>,
          }}
          helperText="10〜300秒の範囲で設定"
          sx={{ mb: 2 }}
        />
        <Slider
          value={timeout}
          onChange={(_, value) => setTimeout(value as number)}
          min={10}
          max={300}
          step={10}
          marks={[
            { value: 10, label: '10秒' },
            { value: 60, label: '60秒' },
            { value: 120, label: '120秒' },
            { value: 180, label: '180秒' },
            { value: 240, label: '240秒' },
            { value: 300, label: '300秒' },
          ]}
        />
        <Box sx={{ display: 'flex', gap: 1, mt: 2, flexWrap: 'wrap' }}>
          <Button variant="outlined" onClick={() => setTimeout(30)} size="small">
            30秒
          </Button>
          <Button variant="outlined" onClick={() => setTimeout(60)} size="small">
            1分
          </Button>
          <Button variant="outlined" onClick={() => setTimeout(120)} size="small">
            2分
          </Button>
          <Button variant="outlined" onClick={() => setTimeout(180)} size="small">
            3分
          </Button>
          <Button variant="outlined" onClick={() => setTimeout(300)} size="small">
            5分
          </Button>
        </Box>
        <Paper sx={{ mt: 2, p: 2, bgcolor: 'grey.50' }}>
          <Typography variant="caption" color="text.secondary">
            💡 GPT APIからの応答を待つ最大時間です。長すぎるとBotの応答が遅くなります。
          </Typography>
        </Paper>
      </Paper>

      {/* Gemini Search タイムアウト */}
      <Paper sx={{ p: 3, mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
          <Timer />
          <Typography variant="h6" fontWeight="bold">
            Gemini Searchタイムアウト
          </Typography>
        </Box>
        <TextField
          type="number"
          value={geminiSearchTimeout}
          onChange={(e) => setGeminiSearchTimeout(Math.max(10, Math.min(600, parseInt(e.target.value) || 10)))}
          fullWidth
          InputProps={{
            endAdornment: <InputAdornment position="end">秒</InputAdornment>,
          }}
          helperText="10〜600秒の範囲で設定"
          sx={{ mb: 2 }}
        />
        <Slider
          value={geminiSearchTimeout}
          onChange={(_, value) => setGeminiSearchTimeout(value as number)}
          min={10}
          max={600}
          marks={[
            { value: 30, label: '30秒' },
            { value: 60, label: '60秒' },
            { value: 120, label: '2分' },
            { value: 180, label: '3分' },
            { value: 300, label: '5分' },
            { value: 600, label: '10分' },
          ]}
        />
        <Box sx={{ display: 'flex', gap: 1, mt: 2, flexWrap: 'wrap' }}>
          <Button variant="outlined" onClick={() => setGeminiSearchTimeout(60)} size="small">
            1分
          </Button>
          <Button variant="outlined" onClick={() => setGeminiSearchTimeout(120)} size="small">
            2分
          </Button>
          <Button variant="outlined" onClick={() => setGeminiSearchTimeout(180)} size="small">
            3分
          </Button>
          <Button variant="outlined" onClick={() => setGeminiSearchTimeout(300)} size="small">
            5分
          </Button>
          <Button variant="outlined" onClick={() => setGeminiSearchTimeout(600)} size="small">
            10分
          </Button>
        </Box>
        <Paper sx={{ mt: 2, p: 2, bgcolor: 'grey.50' }}>
          <Typography variant="caption" color="text.secondary">
            💡 Gemini Web検索の応答を待つ最大時間です。検索が複雑な場合は長めに設定してください。
          </Typography>
        </Paper>
      </Paper>

      {/* 最近のやり取り件数 */}
      <Paper sx={{ p: 3, mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
          <TextFields />
          <Typography variant="h6" fontWeight="bold">
            最近のやり取り件数
          </Typography>
        </Box>
        <TextField
          type="number"
          value={recentContextCount}
          onChange={(e) => setRecentContextCount(Math.max(1, Math.min(100, parseInt(e.target.value) || 1)))}
          fullWidth
          InputProps={{
            endAdornment: <InputAdornment position="end">件</InputAdornment>,
          }}
          helperText="1〜100件の範囲で設定"
          sx={{ mb: 2 }}
        />
        <Slider
          value={recentContextCount}
          onChange={(_, value) => setRecentContextCount(value as number)}
          min={1}
          max={100}
          marks={[
            { value: 1, label: '1件' },
            { value: 25, label: '25件' },
            { value: 50, label: '50件' },
            { value: 75, label: '75件' },
            { value: 100, label: '100件' },
          ]}
        />
        <Box sx={{ display: 'flex', gap: 1, mt: 2, flexWrap: 'wrap' }}>
          <Button variant="outlined" onClick={() => setRecentContextCount(5)} size="small">
            5件
          </Button>
          <Button variant="outlined" onClick={() => setRecentContextCount(10)} size="small">
            10件
          </Button>
          <Button variant="outlined" onClick={() => setRecentContextCount(20)} size="small">
            20件
          </Button>
          <Button variant="outlined" onClick={() => setRecentContextCount(50)} size="small">
            50件
          </Button>
          <Button variant="outlined" onClick={() => setRecentContextCount(100)} size="small">
            100件
          </Button>
        </Box>
        <Paper sx={{ mt: 2, p: 2, bgcolor: 'grey.50' }}>
          <Typography variant="caption" color="text.secondary">
            💡 会話が長くなった時、要約とともに含める直近のやり取りの件数です。多すぎるとトークン消費が増えます。
          </Typography>
        </Paper>
      </Paper>

      {/* 要約開始閾値 */}
      <Paper sx={{ p: 3, mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
          <TextFields />
          <Typography variant="h6" fontWeight="bold">
            要約開始閾値
          </Typography>
        </Box>
        <TextField
          type="number"
          value={summaryThreshold}
          onChange={(e) => setSummaryThreshold(Math.max(1000, Math.min(50000, parseInt(e.target.value) || 1000)))}
          fullWidth
          InputProps={{
            endAdornment: <InputAdornment position="end">文字</InputAdornment>,
          }}
          helperText="1000〜50000文字の範囲で設定"
          sx={{ mb: 2 }}
        />
        <Slider
          value={summaryThreshold}
          onChange={(_, value) => setSummaryThreshold(value as number)}
          min={1000}
          max={50000}
          step={1000}
          marks={[
            { value: 1000, label: '1k' },
            { value: 12500, label: '12.5k' },
            { value: 25000, label: '25k' },
            { value: 37500, label: '37.5k' },
            { value: 50000, label: '50k' },
          ]}
        />
        <Box sx={{ display: 'flex', gap: 1, mt: 2, flexWrap: 'wrap' }}>
          <Button variant="outlined" onClick={() => setSummaryThreshold(3000)} size="small">
            3,000文字
          </Button>
          <Button variant="outlined" onClick={() => setSummaryThreshold(5000)} size="small">
            5,000文字
          </Button>
          <Button variant="outlined" onClick={() => setSummaryThreshold(10000)} size="small">
            10,000文字
          </Button>
          <Button variant="outlined" onClick={() => setSummaryThreshold(20000)} size="small">
            20,000文字
          </Button>
          <Button variant="outlined" onClick={() => setSummaryThreshold(50000)} size="small">
            50,000文字
          </Button>
        </Box>
        <Paper sx={{ mt: 2, p: 2, bgcolor: 'grey.50' }}>
          <Typography variant="caption" color="text.secondary">
            💡 会話履歴がこの文字数を超えると要約を作成します。小さすぎると頻繁に要約が作成され、大きすぎるとトークン消費が増えます。
          </Typography>
        </Paper>
      </Paper>

      {/* 要約最大トークン数 */}
      <Paper sx={{ p: 3, mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
          <TextFields />
          <Typography variant="h6" fontWeight="bold">
            要約最大トークン数
          </Typography>
        </Box>
        <TextField
          type="number"
          value={maxSummaryTokens}
          onChange={(e) => setMaxSummaryTokens(Math.max(1000, Math.min(100000, parseInt(e.target.value) || 1000)))}
          fullWidth
          InputProps={{
            endAdornment: <InputAdornment position="end">トークン</InputAdornment>,
          }}
          helperText="1000〜100000トークンの範囲で設定"
          sx={{ mb: 2 }}
        />
        <Slider
          value={maxSummaryTokens}
          onChange={(_, value) => setMaxSummaryTokens(value as number)}
          min={1000}
          max={100000}
          step={1000}
          marks={[
            { value: 1000, label: '1k' },
            { value: 25000, label: '25k' },
            { value: 50000, label: '50k' },
            { value: 75000, label: '75k' },
            { value: 100000, label: '100k' },
          ]}
        />
        <Box sx={{ display: 'flex', gap: 1, mt: 2, flexWrap: 'wrap' }}>
          <Button variant="outlined" onClick={() => setMaxSummaryTokens(4000)} size="small">
            4,000
          </Button>
          <Button variant="outlined" onClick={() => setMaxSummaryTokens(8000)} size="small">
            8,000
          </Button>
          <Button variant="outlined" onClick={() => setMaxSummaryTokens(16000)} size="small">
            16,000
          </Button>
          <Button variant="outlined" onClick={() => setMaxSummaryTokens(32000)} size="small">
            32,000
          </Button>
          <Button variant="outlined" onClick={() => setMaxSummaryTokens(64000)} size="small">
            64,000
          </Button>
          <Button variant="outlined" onClick={() => setMaxSummaryTokens(100000)} size="small">
            100,000
          </Button>
        </Box>
        <Paper sx={{ mt: 2, p: 2, bgcolor: 'grey.50' }}>
          <Typography variant="caption" color="text.secondary">
            💡 要約作成時にGPT APIに送信できる最大トークン数です。大きすぎるとAPI呼び出しが失敗する可能性があります。GPT-4の場合は8000〜16000程度が推奨です。
          </Typography>
        </Paper>
      </Paper>

      {/* 印象最大文字数 */}
      <Paper sx={{ p: 3, mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
          <Psychology />
          <Typography variant="h6" fontWeight="bold">
            印象最大文字数
          </Typography>
        </Box>
        <TextField
          type="number"
          value={maxImpressionLength}
          onChange={(e) => setMaxImpressionLength(Math.max(50, Math.min(2000, parseInt(e.target.value) || 50)))}
          fullWidth
          InputProps={{
            endAdornment: <InputAdornment position="end">文字</InputAdornment>,
          }}
          helperText="50〜2000文字の範囲で設定"
          sx={{ mb: 2 }}
        />
        <Slider
          value={maxImpressionLength}
          onChange={(_, value) => setMaxImpressionLength(value as number)}
          min={50}
          max={2000}
          step={50}
          marks={[
            { value: 50, label: '50' },
            { value: 500, label: '500' },
            { value: 1000, label: '1000' },
            { value: 1500, label: '1500' },
            { value: 2000, label: '2000' },
          ]}
        />
        <Box sx={{ display: 'flex', gap: 1, mt: 2, flexWrap: 'wrap' }}>
          <Button variant="outlined" onClick={() => setMaxImpressionLength(200)} size="small">
            簡潔 (200文字)
          </Button>
          <Button variant="outlined" onClick={() => setMaxImpressionLength(500)} size="small">
            標準 (500文字)
          </Button>
          <Button variant="outlined" onClick={() => setMaxImpressionLength(1000)} size="small">
            詳細 (1000文字)
          </Button>
          <Button variant="outlined" onClick={() => setMaxImpressionLength(2000)} size="small">
            最大 (2000文字)
          </Button>
        </Box>
        <Paper sx={{ mt: 2, p: 2, bgcolor: 'grey.50' }}>
          <Typography variant="caption" color="text.secondary">
            💡 Botが各ユーザーに対して記録する印象の最大文字数です。ユーザーの性格や興味関心、会話履歴から得た情報を記録し、よりパーソナライズされた返信に活用します。
          </Typography>
        </Paper>
      </Paper>

      {/* 心境最大文字数 */}
      <Paper sx={{ p: 3, mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
          <Psychology />
          <Typography variant="h6" fontWeight="bold">
            心境最大文字数
          </Typography>
        </Box>
        <TextField
          type="number"
          value={maxMentalDiaryLength}
          onChange={(e) => setMaxMentalDiaryLength(Math.max(100, Math.min(5000, parseInt(e.target.value) || 100)))}
          fullWidth
          InputProps={{
            endAdornment: <InputAdornment position="end">文字</InputAdornment>,
          }}
          helperText="100〜5000文字の範囲で設定"
          sx={{ mb: 2 }}
        />
        <Slider
          value={maxMentalDiaryLength}
          onChange={(_, value) => setMaxMentalDiaryLength(value as number)}
          min={100}
          max={5000}
          step={100}
          marks={[
            { value: 100, label: '100' },
            { value: 1250, label: '1.25k' },
            { value: 2500, label: '2.5k' },
            { value: 3750, label: '3.75k' },
            { value: 5000, label: '5k' },
          ]}
        />
        <Box sx={{ display: 'flex', gap: 1, mt: 2, flexWrap: 'wrap' }}>
          <Button variant="outlined" onClick={() => setMaxMentalDiaryLength(500)} size="small">
            簡潔 (500文字)
          </Button>
          <Button variant="outlined" onClick={() => setMaxMentalDiaryLength(1000)} size="small">
            標準 (1000文字)
          </Button>
          <Button variant="outlined" onClick={() => setMaxMentalDiaryLength(2000)} size="small">
            詳細 (2000文字)
          </Button>
          <Button variant="outlined" onClick={() => setMaxMentalDiaryLength(3000)} size="small">
            長め (3000文字)
          </Button>
          <Button variant="outlined" onClick={() => setMaxMentalDiaryLength(5000)} size="small">
            最大 (5000文字)
          </Button>
        </Box>
        <Paper sx={{ mt: 2, p: 2, bgcolor: 'grey.50' }}>
          <Typography variant="caption" color="text.secondary">
            💡 Botの内面的な心境（気分、好きな人、興味、目標など）を記録する最大文字数です。会話を通じてBotの人格が成長し、より自然で人間らしい返信が可能になります。
          </Typography>
        </Paper>
      </Paper>

      {/* 設定例 */}
      <Paper sx={{ p: 3, bgcolor: 'info.light' }}>
        <Typography variant="subtitle1" fontWeight="bold" gutterBottom>
          現在の設定
        </Typography>
        <Typography variant="body1" gutterBottom>
          Botは<strong>約{answerLength}文字</strong>の返信を生成し、
          GPT APIは<strong>{timeout}秒</strong>でタイムアウトします。
        </Typography>
        <Typography variant="body1" gutterBottom>
          Gemini Searchは<strong>{geminiSearchTimeout}秒</strong>でタイムアウトします。
        </Typography>
        <Typography variant="body1" gutterBottom>
          会話要約時には直近<strong>{recentContextCount}件</strong>のやり取りを含めます。
        </Typography>
        <Typography variant="body1" gutterBottom>
          会話履歴が<strong>{summaryThreshold}文字</strong>を超えると要約を作成します。
        </Typography>
        <Typography variant="body1" gutterBottom>
          要約作成時の最大トークン数は<strong>{maxSummaryTokens}トークン</strong>です。
        </Typography>
        <Typography variant="body1" gutterBottom>
          ユーザー印象は<strong>{maxImpressionLength}文字</strong>以内で記録します。
        </Typography>
        <Typography variant="body1">
          Bot心境は<strong>{maxMentalDiaryLength}文字</strong>以内で記録します。
        </Typography>
      </Paper>
    </Container>
  );
};

