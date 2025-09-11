use nih_plug::prelude::*;
use std::sync::Arc;

struct VST_test1 {
    params: Arc<VST_test1Params>,


    /*
    paramsとはパラメータのこと、ここではArc<VST_test1Params>型のparamsフィールドを持つ構造体を定義している。

    ArcはAtomic Reference Countedの略で、複数のスレッド間で安全に共有できる参照カウント付きスマートポインタを提供する。
    これにより、VST_test1構造体のインスタンスが複数の場所で共有されても、メモリ管理が自動的に行われる。

    VST_test1Paramsは後で定義される構造体で、プラグインのパラメータを格納するために使用される。（重要）
    
     */
}

#[derive(Params)]
struct VST_test1Params {
    #[id = "gain"]
    pub gain: FloatParam,

    /*
    #[derive(Params)]はNih-plugに「ここにパラメーターがありますよ」と指定する。

    
    
     */
}

impl Default for VST_test1 {
    fn default() -> Self {
        Self {
            params: Arc::new(VST_test1Params::default()),
        }
    }
}

impl Default for VST_test1Params {

    fn default() -> Self {
        Self {
            gain: FloatParam::new(
                "Gain",                                                          //パラメータの名前
                util::db_to_gain(0.0),                                    //デフォルト値(0dB)
                FloatRange::Skewed {                                            //パラメータの範囲とスキューを指定
                    min: util::db_to_gain(-30.0),                                 //最小値(-30dB)
                    max: util::db_to_gain(30.0),                                  //最大値(30dB)
                
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0), //スキューの計算
                },
            )
           
            .with_smoother(SmoothingStyle::Logarithmic(50.0))                   //スムーザーを追加(50ms)
            .with_unit(" dB")                                                          //単位をdBに設定
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))  //値を文字列に変換するフォーマッターを設定
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),                    //文字列を値に変換するフォーマッターを設定
        }
    }
}

impl Plugin for VST_test1 {

    //プラグインの「名前」「作者」「URL」「バージョン」などを設定。

    const NAME    : &'static str = "VST_test1";                  //プラグインの名前
    const VENDOR  : &'static str = "VST_test1";                  //プラグインの作者
    const URL     : &'static str = env!("CARGO_PKG_HOMEPAGE");   //プラグインのURL
    const EMAIL   : &'static str = "your@email.com";             //プラグインの作者のメールアドレス
    const VERSION : &'static str = env!("CARGO_PKG_VERSION");    //プラグインのバージョン

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {

        main_input_channels  : NonZeroU32::new(2),          //入力(２チャンネルなのでステレオ)
        main_output_channels : NonZeroU32::new(2),          //出力(２チャンネルなのでステレオ)

        aux_input_ports      : &[],                         //AUX入力ポート(今回は無し)
        aux_output_ports     : &[],                         //AUX出力ポート(今回は無し)      AUXはイヤホンのやつ

        names                : PortNames::const_default(),  //ポートの名前(今回はデフォルト)

    }];


    const MIDI_INPUT   : MidiConfig = MidiConfig::None;  //MIDI入力無し
    const MIDI_OUTPUT  : MidiConfig = MidiConfig::None;  //MIDI出力無し

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;       //サンプル精度のオートメーションをサポート

    type SysExMessage   = ();   //SysExメッセージ無し
    type BackgroundTask = ();   //バックグラウンドタスク無し

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        
        true
    }

    fn reset(&mut self) {
        
    }

    fn process(
        &mut self,

        buffer   : &mut Buffer,
        _aux     : &mut AuxiliaryBuffers,
        _context : &mut impl ProcessContext<Self>,

    ) -> ProcessStatus {
        
        for channel_samples in buffer.iter_samples() {
            
            let gain = self.params.gain.smoothed.next();

            for sample in channel_samples {
                *sample *= gain;
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for VST_test1 {

    //CLAP用の設定、さっきと同じ

    const CLAP_ID           : &'static str = "com.your-domain.VST-test1";
    const CLAP_DESCRIPTION  : Option<&'static str> = Some("A short description of your plugin");
    const CLAP_MANUAL_URL   : Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL  : Option<&'static str> = None;

    const CLAP_FEATURES     : &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

impl Vst3Plugin for VST_test1 {
    const VST3_CLASS_ID: [u8; 16] = *b"Exactly16Chars!!";  //VST3のクラスID、16文字である必要がある(大事)
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics]; //VST3のサブカテゴリ
}

nih_export_clap!(VST_test1);
nih_export_vst3!(VST_test1);
