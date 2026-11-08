//! SpeechCraft Internationalization (i18n) Engine.
//!
//! Follows the fleet standard: typed locale enums, zero-panic string lookups,
//! multi-language translations (EN, DE, FR, ES, JA, KO, ZH),
//! covering wake-word orchestration, voice command bus, acoustic prosody, and timeline mastering.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AppLocale {
    #[default]
    En, // English (Global Default)
    De, // German
    Fr, // French
    Es, // Spanish
    Ja, // Japanese
    Ko, // Korean
    Zh, // Chinese
}

impl AppLocale {
    pub fn all() -> &'static [AppLocale] {
        &[
            AppLocale::En,
            AppLocale::De,
            AppLocale::Fr,
            AppLocale::Es,
            AppLocale::Ja,
            AppLocale::Ko,
            AppLocale::Zh,
        ]
    }

    pub fn code(&self) -> &'static str {
        match self {
            AppLocale::En => "en",
            AppLocale::De => "de",
            AppLocale::Fr => "fr",
            AppLocale::Es => "es",
            AppLocale::Ja => "ja",
            AppLocale::Ko => "ko",
            AppLocale::Zh => "zh",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AppLocale::En => "English (US/UK)",
            AppLocale::De => "Deutsch (DE/AT/CH)",
            AppLocale::Fr => "Français (FR)",
            AppLocale::Es => "Español (ES/LATAM)",
            AppLocale::Ja => "日本語 (JA)",
            AppLocale::Ko => "한국어 (KO)",
            AppLocale::Zh => "中文 (ZH)",
        }
    }

    pub fn flag(&self) -> &'static str {
        match self {
            AppLocale::En => "🇬🇧",
            AppLocale::De => "🇩🇪",
            AppLocale::Fr => "🇫🇷",
            AppLocale::Es => "🇪🇸",
            AppLocale::Ja => "🇯🇵",
            AppLocale::Ko => "🇰🇷",
            AppLocale::Zh => "🇨🇳",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum I18nKey {
    // Menu Bar Top Items
    File,
    Studio,
    VoiceBus,
    Acoustics,
    Security,
    Help,

    // Studio Screens
    Cockpit,
    ProsodySculptor,
    SpokenTimeline,
    VoiceprintVault,

    // Wake Word & Ingress
    WakeWord,
    Greeting,
    SleepWord,
    Listening,
    Dormant,
    StartListening,
    StopListening,

    // Actions & Tools
    SpeakText,
    TranscribeAudio,
    ExportBroadcastMaster,
    SetTimer,

    // Hardware & Voice Bus Controls
    HardwareIngress,
    InputDevice,
    StartMic,
    StopMic,
    Refresh,
    VoiceCommandBusTitle,
    DispatchToFleet,
    ZoomLabel,

    // Status & Readiness
    Authenticated,
    Guest,
    StatusReady,
}

pub fn t(key: I18nKey, locale: AppLocale) -> &'static str {
    match (key, locale) {
        // Menu Bar Top Items
        (I18nKey::File, AppLocale::En) => "File",
        (I18nKey::File, AppLocale::De) => "Datei",
        (I18nKey::File, AppLocale::Fr) => "Fichier",
        (I18nKey::File, AppLocale::Es) => "Archivo",
        (I18nKey::File, AppLocale::Ja) => "ファイル",
        (I18nKey::File, AppLocale::Ko) => "파일",
        (I18nKey::File, AppLocale::Zh) => "文件",

        (I18nKey::Studio, AppLocale::En) => "Studio",
        (I18nKey::Studio, AppLocale::De) => "Studio",
        (I18nKey::Studio, AppLocale::Fr) => "Studio",
        (I18nKey::Studio, AppLocale::Es) => "Estudio",
        (I18nKey::Studio, AppLocale::Ja) => "スタジオ",
        (I18nKey::Studio, AppLocale::Ko) => "스튜디오",
        (I18nKey::Studio, AppLocale::Zh) => "演播室",

        (I18nKey::VoiceBus, AppLocale::En) => "Voice Bus",
        (I18nKey::VoiceBus, AppLocale::De) => "Sprach-Bus",
        (I18nKey::VoiceBus, AppLocale::Fr) => "Bus vocal",
        (I18nKey::VoiceBus, AppLocale::Es) => "Bus de voz",
        (I18nKey::VoiceBus, AppLocale::Ja) => "ボイスコマンド・バス",
        (I18nKey::VoiceBus, AppLocale::Ko) => "음성 명령 버스",
        (I18nKey::VoiceBus, AppLocale::Zh) => "语音指令总线",

        (I18nKey::Acoustics, AppLocale::En) => "Acoustics",
        (I18nKey::Acoustics, AppLocale::De) => "Akustik",
        (I18nKey::Acoustics, AppLocale::Fr) => "Acoustique",
        (I18nKey::Acoustics, AppLocale::Es) => "Acústica",
        (I18nKey::Acoustics, AppLocale::Ja) => "音響・韻律",
        (I18nKey::Acoustics, AppLocale::Ko) => "음향 및 운율",
        (I18nKey::Acoustics, AppLocale::Zh) => "声学与韵律",

        (I18nKey::Security, AppLocale::En) => "Security",
        (I18nKey::Security, AppLocale::De) => "Sicherheit",
        (I18nKey::Security, AppLocale::Fr) => "Sécurité",
        (I18nKey::Security, AppLocale::Es) => "Seguridad",
        (I18nKey::Security, AppLocale::Ja) => "セキュリティ",
        (I18nKey::Security, AppLocale::Ko) => "보안",
        (I18nKey::Security, AppLocale::Zh) => "声纹安全与鉴权",

        (I18nKey::Help, AppLocale::En) => "Help",
        (I18nKey::Help, AppLocale::De) => "Hilfe",
        (I18nKey::Help, AppLocale::Fr) => "Aide",
        (I18nKey::Help, AppLocale::Es) => "Ayuda",
        (I18nKey::Help, AppLocale::Ja) => "ヘルプ",
        (I18nKey::Help, AppLocale::Ko) => "도움말",
        (I18nKey::Help, AppLocale::Zh) => "帮助与文档",

        // Studio Screens
        (I18nKey::Cockpit, AppLocale::En) => "Ambient Voice Cockpit",
        (I18nKey::Cockpit, AppLocale::De) => "Ambientes Sprach-Cockpit",
        (I18nKey::Cockpit, AppLocale::Fr) => "Cockpit vocal ambiant",
        (I18nKey::Cockpit, AppLocale::Es) => "Cabina de voz ambiental",
        (I18nKey::Cockpit, AppLocale::Ja) => "環境対話コックピット",
        (I18nKey::Cockpit, AppLocale::Ko) => "앰비언트 음성 조종석",
        (I18nKey::Cockpit, AppLocale::Zh) => "实时环境拾音座舱",

        (I18nKey::ProsodySculptor, AppLocale::En) => "Prosody & Phoneme Sculptor",
        (I18nKey::ProsodySculptor, AppLocale::De) => "Prosodie & Phonem-Skulpteur",
        (I18nKey::ProsodySculptor, AppLocale::Fr) => "Sculpteur de prosodie et phonèmes",
        (I18nKey::ProsodySculptor, AppLocale::Es) => "Escultor de prosodia y fonemas",
        (I18nKey::ProsodySculptor, AppLocale::Ja) => "韻律・音素スカルプター",
        (I18nKey::ProsodySculptor, AppLocale::Ko) => "운율 및 음소 조각기",
        (I18nKey::ProsodySculptor, AppLocale::Zh) => "韵律起伏与音素雕刻台",

        (I18nKey::SpokenTimeline, AppLocale::En) => "Spoken Literature Timeline",
        (I18nKey::SpokenTimeline, AppLocale::De) => "Gesprochene Literatur-Zeitleiste",
        (I18nKey::SpokenTimeline, AppLocale::Fr) => "Chronologie de littérature parlée",
        (I18nKey::SpokenTimeline, AppLocale::Es) => "Línea de tiempo de literatura hablada",
        (I18nKey::SpokenTimeline, AppLocale::Ja) => "朗読・ポッドキャスト年表",
        (I18nKey::SpokenTimeline, AppLocale::Ko) => "음성 문학 타임라인",
        (I18nKey::SpokenTimeline, AppLocale::Zh) => "多幕长篇有声书工程轨",

        (I18nKey::VoiceprintVault, AppLocale::En) => "Voiceprint Biometric Vault",
        (I18nKey::VoiceprintVault, AppLocale::De) => "Stimmabdruck-Biometrie-Tresor",
        (I18nKey::VoiceprintVault, AppLocale::Fr) => "Coffre-fort biométrique vocal",
        (I18nKey::VoiceprintVault, AppLocale::Es) => "Bóveda biométrica de voz",
        (I18nKey::VoiceprintVault, AppLocale::Ja) => "声紋バイオメトリック・金庫",
        (I18nKey::VoiceprintVault, AppLocale::Ko) => "음성 생체인식 금고",
        (I18nKey::VoiceprintVault, AppLocale::Zh) => "声纹生物识别自主保险库",

        // Wake Word & Ingress
        (I18nKey::WakeWord, AppLocale::En) => "Wake Word",
        (I18nKey::WakeWord, AppLocale::De) => "Aktivierungswort",
        (I18nKey::WakeWord, AppLocale::Fr) => "Mot d'activation",
        (I18nKey::WakeWord, AppLocale::Es) => "Palabra de activación",
        (I18nKey::WakeWord, AppLocale::Ja) => "起動ワード (ウェイクワード)",
        (I18nKey::WakeWord, AppLocale::Ko) => "호출어",
        (I18nKey::WakeWord, AppLocale::Zh) => "唤醒词",

        (I18nKey::Greeting, AppLocale::En) => "Spoken Greeting",
        (I18nKey::Greeting, AppLocale::De) => "Gesprochene Begrüßung",
        (I18nKey::Greeting, AppLocale::Fr) => "Salutation vocale",
        (I18nKey::Greeting, AppLocale::Es) => "Saludo hablado",
        (I18nKey::Greeting, AppLocale::Ja) => "応答メッセージ",
        (I18nKey::Greeting, AppLocale::Ko) => "음성 인사말",
        (I18nKey::Greeting, AppLocale::Zh) => "应答问候语",

        (I18nKey::SleepWord, AppLocale::En) => "Sleep Word",
        (I18nKey::SleepWord, AppLocale::De) => "Schlafbefehl",
        (I18nKey::SleepWord, AppLocale::Fr) => "Mot de veille",
        (I18nKey::SleepWord, AppLocale::Es) => "Palabra de reposo",
        (I18nKey::SleepWord, AppLocale::Ja) => "休止コマンド",
        (I18nKey::SleepWord, AppLocale::Ko) => "수면 단어",
        (I18nKey::SleepWord, AppLocale::Zh) => "休眠指令词",

        (I18nKey::Listening, AppLocale::En) => "Listening (VAD Active)",
        (I18nKey::Listening, AppLocale::De) => "Lauscht (VAD aktiv)",
        (I18nKey::Listening, AppLocale::Fr) => "À l'écoute (VAD actif)",
        (I18nKey::Listening, AppLocale::Es) => "Escuchando (VAD activo)",
        (I18nKey::Listening, AppLocale::Ja) => "待機中 (VAD検知稼働)",
        (I18nKey::Listening, AppLocale::Ko) => "듣는 중 (VAD 활성)",
        (I18nKey::Listening, AppLocale::Zh) => "正在倾听 (VAD活动中)",

        (I18nKey::Dormant, AppLocale::En) => "Dormant / Muted",
        (I18nKey::Dormant, AppLocale::De) => "Inaktiv / Stumm",
        (I18nKey::Dormant, AppLocale::Fr) => "Inactif / Muet",
        (I18nKey::Dormant, AppLocale::Es) => "Inactivo / Silenciado",
        (I18nKey::Dormant, AppLocale::Ja) => "休止 / ミュート",
        (I18nKey::Dormant, AppLocale::Ko) => "비활성 / 음소거",
        (I18nKey::Dormant, AppLocale::Zh) => "静音休眠",

        (I18nKey::StartListening, AppLocale::En) => "Start Wake Listener",
        (I18nKey::StartListening, AppLocale::De) => "Aktivierungs-Lauscher starten",
        (I18nKey::StartListening, AppLocale::Fr) => "Démarrer l'écoute d'activation",
        (I18nKey::StartListening, AppLocale::Es) => "Iniciar escucha de activación",
        (I18nKey::StartListening, AppLocale::Ja) => "リスナーを開始",
        (I18nKey::StartListening, AppLocale::Ko) => "리스너 시작",
        (I18nKey::StartListening, AppLocale::Zh) => "启动唤醒监听",

        (I18nKey::StopListening, AppLocale::En) => "Stop Wake Listener",
        (I18nKey::StopListening, AppLocale::De) => "Aktivierungs-Lauscher stoppen",
        (I18nKey::StopListening, AppLocale::Fr) => "Arrêter l'écoute d'activation",
        (I18nKey::StopListening, AppLocale::Es) => "Detener escucha de activación",
        (I18nKey::StopListening, AppLocale::Ja) => "リスナーを停止",
        (I18nKey::StopListening, AppLocale::Ko) => "리스너 중지",
        (I18nKey::StopListening, AppLocale::Zh) => "停止唤醒监听",

        // Actions & Tools
        (I18nKey::SpeakText, AppLocale::En) => "Synthesize Voice (TTS)",
        (I18nKey::SpeakText, AppLocale::De) => "Sprache synthetisieren (TTS)",
        (I18nKey::SpeakText, AppLocale::Fr) => "Synthétiser la voix (TTS)",
        (I18nKey::SpeakText, AppLocale::Es) => "Sintetizar voz (TTS)",
        (I18nKey::SpeakText, AppLocale::Ja) => "音声合成出力 (TTS)",
        (I18nKey::SpeakText, AppLocale::Ko) => "음성 합성 (TTS)",
        (I18nKey::SpeakText, AppLocale::Zh) => "执行语音合成 (TTS)",

        (I18nKey::TranscribeAudio, AppLocale::En) => "Transcribe Utterance (STT)",
        (I18nKey::TranscribeAudio, AppLocale::De) => "Sprache transkribieren (STT)",
        (I18nKey::TranscribeAudio, AppLocale::Fr) => "Transcrire la parole (STT)",
        (I18nKey::TranscribeAudio, AppLocale::Es) => "Transcribir audio (STT)",
        (I18nKey::TranscribeAudio, AppLocale::Ja) => "音声認識 (STT)",
        (I18nKey::TranscribeAudio, AppLocale::Ko) => "음성 전사 (STT)",
        (I18nKey::TranscribeAudio, AppLocale::Zh) => "语音识别转录 (STT)",

        (I18nKey::ExportBroadcastMaster, AppLocale::En) => "Export Broadcast Master (-16 LUFS)",
        (I18nKey::ExportBroadcastMaster, AppLocale::De) => {
            "Broadcast-Master exportieren (-16 LUFS)"
        }
        (I18nKey::ExportBroadcastMaster, AppLocale::Fr) => {
            "Exporter le master de diffusion (-16 LUFS)"
        }
        (I18nKey::ExportBroadcastMaster, AppLocale::Es) => "Exportar máster de emisión (-16 LUFS)",
        (I18nKey::ExportBroadcastMaster, AppLocale::Ja) => "放送基準マスター出力 (-16 LUFS)",
        (I18nKey::ExportBroadcastMaster, AppLocale::Ko) => "방송용 마스터 내보내기 (-16 LUFS)",
        (I18nKey::ExportBroadcastMaster, AppLocale::Zh) => "导出全保真广播级母带 (-16 LUFS)",

        (I18nKey::SetTimer, AppLocale::En) => "Set Audible Timer",
        (I18nKey::SetTimer, AppLocale::De) => "Akustischen Timer stellen",
        (I18nKey::SetTimer, AppLocale::Fr) => "Régler une minuterie sonore",
        (I18nKey::SetTimer, AppLocale::Es) => "Establecer temporizador audible",
        (I18nKey::SetTimer, AppLocale::Ja) => "音声タイマー設定",
        (I18nKey::SetTimer, AppLocale::Ko) => "음성 타이머 설정",
        (I18nKey::SetTimer, AppLocale::Zh) => "设定语音报时定时器",

        // Hardware & Voice Bus Controls
        (I18nKey::HardwareIngress, AppLocale::En) => "Hardware Audio Ingress & Microphone Stream",
        (I18nKey::HardwareIngress, AppLocale::De) => "Hardware-Audio-Eingang & Mikrofon-Stream",
        (I18nKey::HardwareIngress, AppLocale::Fr) => "Entrée audio matérielle & flux microphone",
        (I18nKey::HardwareIngress, AppLocale::Es) => {
            "Entrada de audio hardware y flujo de micrófono"
        }
        (I18nKey::HardwareIngress, AppLocale::Ja) => "ハードウェア音声入力 & マイクストリーム",
        (I18nKey::HardwareIngress, AppLocale::Ko) => "하드웨어 오디오 입력 및 마이크 스트림",
        (I18nKey::HardwareIngress, AppLocale::Zh) => "硬件音频输入与麦克风流",

        (I18nKey::InputDevice, AppLocale::En) => "Input Device",
        (I18nKey::InputDevice, AppLocale::De) => "Eingabegerät",
        (I18nKey::InputDevice, AppLocale::Fr) => "Périphérique d'entrée",
        (I18nKey::InputDevice, AppLocale::Es) => "Dispositivo de entrada",
        (I18nKey::InputDevice, AppLocale::Ja) => "入力デバイス",
        (I18nKey::InputDevice, AppLocale::Ko) => "입력 장치",
        (I18nKey::InputDevice, AppLocale::Zh) => "音频输入设备",

        (I18nKey::StartMic, AppLocale::En) => "🎙️ Start Continuous Mic",
        (I18nKey::StartMic, AppLocale::De) => "🎙️ Mikrofon-Aufnahme starten",
        (I18nKey::StartMic, AppLocale::Fr) => "🎙️ Démarrer l'écoute micro",
        (I18nKey::StartMic, AppLocale::Es) => "🎙️ Iniciar captura de micrófono",
        (I18nKey::StartMic, AppLocale::Ja) => "🎙️ マイク入力を開始",
        (I18nKey::StartMic, AppLocale::Ko) => "🎙️ 마이크 스트림 시작",
        (I18nKey::StartMic, AppLocale::Zh) => "🎙️ 启动麦克风持续拾音",

        (I18nKey::StopMic, AppLocale::En) => "⏹️ Stop Microphone",
        (I18nKey::StopMic, AppLocale::De) => "⏹️ Mikrofon stoppen",
        (I18nKey::StopMic, AppLocale::Fr) => "⏹️ Arrêter le microphone",
        (I18nKey::StopMic, AppLocale::Es) => "⏹️ Detener micrófono",
        (I18nKey::StopMic, AppLocale::Ja) => "⏹️ マイクを停止",
        (I18nKey::StopMic, AppLocale::Ko) => "⏹️ 마이크 중지",
        (I18nKey::StopMic, AppLocale::Zh) => "⏹️ 停止麦克风",

        (I18nKey::Refresh, AppLocale::En) => "🔄 Refresh Devices",
        (I18nKey::Refresh, AppLocale::De) => "🔄 Geräte aktualisieren",
        (I18nKey::Refresh, AppLocale::Fr) => "🔄 Rafraîchir les périphériques",
        (I18nKey::Refresh, AppLocale::Es) => "🔄 Actualizar dispositivos",
        (I18nKey::Refresh, AppLocale::Ja) => "🔄 デバイス再検出",
        (I18nKey::Refresh, AppLocale::Ko) => "🔄 장치 새로고침",
        (I18nKey::Refresh, AppLocale::Zh) => "🔄 刷新设备列表",

        (I18nKey::VoiceCommandBusTitle, AppLocale::En) => "Voice Command Bus (Router: 10996)",
        (I18nKey::VoiceCommandBusTitle, AppLocale::De) => "Sprachbefehl-Bus (Router: 10996)",
        (I18nKey::VoiceCommandBusTitle, AppLocale::Fr) => {
            "Bus de commandes vocales (Routeur: 10996)"
        }
        (I18nKey::VoiceCommandBusTitle, AppLocale::Es) => {
            "Bus de comandos de voz (Enrutador: 10996)"
        }
        (I18nKey::VoiceCommandBusTitle, AppLocale::Ja) => "音声コマンドバス (ルーター: 10996)",
        (I18nKey::VoiceCommandBusTitle, AppLocale::Ko) => "음성 명령 버스 (라우터: 10996)",
        (I18nKey::VoiceCommandBusTitle, AppLocale::Zh) => "语音指令总线 (路由: 10996)",

        (I18nKey::DispatchToFleet, AppLocale::En) => "🚀 Dispatch to Fleet Agent",
        (I18nKey::DispatchToFleet, AppLocale::De) => "🚀 An Flotten-Agent senden",
        (I18nKey::DispatchToFleet, AppLocale::Fr) => "🚀 Transmettre à l'agent de flotte",
        (I18nKey::DispatchToFleet, AppLocale::Es) => "🚀 Despachar a agente de flota",
        (I18nKey::DispatchToFleet, AppLocale::Ja) => "🚀 フリートエージェントへ送信",
        (I18nKey::DispatchToFleet, AppLocale::Ko) => "🚀 함대 에이전트로 전송",
        (I18nKey::DispatchToFleet, AppLocale::Zh) => "🚀 发送至舰队智能体",

        (I18nKey::ZoomLabel, AppLocale::En) => "Zoom",
        (I18nKey::ZoomLabel, AppLocale::De) => "Zoom",
        (I18nKey::ZoomLabel, AppLocale::Fr) => "Zoom",
        (I18nKey::ZoomLabel, AppLocale::Es) => "Zoom",
        (I18nKey::ZoomLabel, AppLocale::Ja) => "ズーム",
        (I18nKey::ZoomLabel, AppLocale::Ko) => "확대/축소",
        (I18nKey::ZoomLabel, AppLocale::Zh) => "视口缩放",

        // Status & Readiness
        (I18nKey::Authenticated, AppLocale::En) => "Sovereign Voice Operator",
        (I18nKey::Authenticated, AppLocale::De) => "Souveräner Sprach-Operator",
        (I18nKey::Authenticated, AppLocale::Fr) => "Opérateur vocal souverain",
        (I18nKey::Authenticated, AppLocale::Es) => "Operador de voz soberano",
        (I18nKey::Authenticated, AppLocale::Ja) => "認証済みオペレーター",
        (I18nKey::Authenticated, AppLocale::Ko) => "주권 음성 오퍼레이터",
        (I18nKey::Authenticated, AppLocale::Zh) => "已通过声纹认证主控",

        (I18nKey::Guest, AppLocale::En) => "Guest / Demo Mode",
        (I18nKey::Guest, AppLocale::De) => "Gast / Demo-Modus",
        (I18nKey::Guest, AppLocale::Fr) => "Invité / Mode démo",
        (I18nKey::Guest, AppLocale::Es) => "Invitado / Modo demo",
        (I18nKey::Guest, AppLocale::Ja) => "ゲスト / デモモード",
        (I18nKey::Guest, AppLocale::Ko) => "게스트 / 데모 모드",
        (I18nKey::Guest, AppLocale::Zh) => "访客演示模式",

        (I18nKey::StatusReady, AppLocale::En) => "SpeechCraft Acoustic Studio Ready (Port 14132)",
        (I18nKey::StatusReady, AppLocale::De) => "SpeechCraft Akustik-Studio bereit (Port 14132)",
        (I18nKey::StatusReady, AppLocale::Fr) => "Studio acoustique SpeechCraft prêt (Port 14132)",
        (I18nKey::StatusReady, AppLocale::Es) => {
            "Estudio acústico SpeechCraft listo (Puerto 14132)"
        }
        (I18nKey::StatusReady, AppLocale::Ja) => "SpeechCraft 音響スタジオ 準備完了 (ポート 14132)",
        (I18nKey::StatusReady, AppLocale::Ko) => "SpeechCraft 음향 스튜디오 준비완료 (포트 14132)",
        (I18nKey::StatusReady, AppLocale::Zh) => "SpeechCraft 终极声学工作室已就绪 (端口 14132)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_speechcraft_i18n_locales() {
        assert_eq!(AppLocale::all().len(), 7);
        for loc in AppLocale::all() {
            assert!(!loc.code().is_empty());
            assert!(!loc.display_name().is_empty());
            assert!(!loc.flag().is_empty());
            assert!(!t(I18nKey::File, *loc).is_empty());
            assert!(!t(I18nKey::Cockpit, *loc).is_empty());
            assert!(!t(I18nKey::WakeWord, *loc).is_empty());
            assert!(!t(I18nKey::SpeakText, *loc).is_empty());
            assert!(!t(I18nKey::HardwareIngress, *loc).is_empty());
            assert!(!t(I18nKey::StartMic, *loc).is_empty());
            assert!(!t(I18nKey::ZoomLabel, *loc).is_empty());
            assert!(!t(I18nKey::StatusReady, *loc).is_empty());
        }
    }
}
