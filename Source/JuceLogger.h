// JuceLogger.h (新規作成)
#pragma once
#include "RustBridge.h"
#include <string>
#include <atomic>

namespace Logger
{
    // RustBridgeへのポインタを保持するグローバルなatomic変数
    inline std::atomic<RustBridge*> g_rustBridge = nullptr;

    // AudioProcessorのコンストラクタで呼ばれる初期化関数
    inline void init(RustBridge* bridge)
    {
        g_rustBridge.store(bridge);
    }

    // AudioProcessorのデストラクタで呼ばれる終了関数
    inline void shutdown()
    {
        g_rustBridge.store(nullptr);
    }

    // どこからでも呼び出せるログ関数
    inline void log(const std::string& message)
    {
        auto bridge = g_rustBridge.load();
        if (bridge)
        {
            bridge->log(message);
        }
    }
}  