//RustBridge.h

#pragma once

// Windows �� DLL API ���g�����߂� windows.h ���C���N���[�h
#include <windows.h>
#include <string>

/*
  ParamBundle �̃��C�A�E�g�� Rust ���� #[repr(C)] ParamBundle �� **���S�Ɉ�v** ������K�v������܂��B
  �����E�^�i�����ł� float�j����v���Ă��Ȃ��Ɩ���`����ɂȂ�܂��B
*/
struct ParamBundle {
    float attack;
    float decay;
    float sustain;
    float release;
    float blend;
    float cutoff;
    float resonance;
};

/*
  RustBridge �N���X
  - DLL �̃��[�h�^�֐��V���{���̉���
  - Rust �� ctx�i�s�����|�C���^�j��ێ�
  - JUCE ������͂��̃N���X��ʂ��� Rust ���Ăяo��
*/
class RustBridge {
public:
    RustBridge();
    ~RustBridge();

    // DLL �̃��[�h / �A�����[�h
    bool load(const char* dllPath);
    void unload();

    // �R���e�L�X�g�쐬 / �j���iRust ���ɃC���X�^���X���쐬������j
    bool create_context(float sample_rate, int block_size, int num_channels);
    void destroy_context();

    // ��́iblocking�F�Ăяo�����Ńo�b�N�O���E���h�X���b�h�Ŏg�����Ɓj
    int analyzeFile(const char* path);

    // �p�����[�^�ꊇ�ݒ�iParamBundle ���\���̂œn���j
    void set_params(const ParamBundle& p);

    // MIDI note on/off �V���v���C���^�[�t�F�[�X
    void note_on(int note, int velocity);
    void note_off(int note);

    // �I�[�f�B�I�����F�`�����l��0 �̃|�C���^�� num_samples ���� PCM ���������ޑz��
    // �i�Ăяo�����ő��`�����l���փR�s�[��������ɂ��Ă���j
    void process(float* out_buffer_channel0, int num_samples, int num_channels);

    void log(const std::string& message);

private:
    HMODULE dll;   // DLL �n���h��
    void* ctx;     // Rust ���ō��ꂽ�R���e�L�X�g�i�s�����|�C���^�j

    // FFI �֐��|�C���^�^�iRust ���̃V���{�����ɍ��킹��j
    typedef void*   (*FnCreateCtx)(float, int, int);
    typedef void    (*FnDestroyCtx)(void*);
    typedef int     (*FnAnalyzeFile)(void*, const char*);
    typedef void    (*FnSetParams)(void*, const ParamBundle*);
    typedef void    (*FnNoteOn)(void*, int, int);
    typedef void    (*FnNoteOff)(void*, int);
    typedef void    (*FnProcess)(void*, float*, int, int);
    typedef void    (*FnLogMessage)(const char*);

    // ���ۂ̊֐��|�C���^
    FnCreateCtx    fn_create_ctx;
    FnDestroyCtx   fn_destroy_ctx;
    FnAnalyzeFile  fn_analyze_file;
    FnSetParams    fn_set_params;
    FnNoteOn       fn_note_on;
    FnNoteOff      fn_note_off;
    FnProcess      fn_process;
    FnLogMessage   fn_log_message;
    
};
