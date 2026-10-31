// Emberstone 差分測試用：取代 effolkronium::random_static（case R1–R3）。
// 編譯 RosettaStone 與橋接程式時把這個目錄放在 include 路徑最前面，
// RosettaStone 所有 `Random::...` 呼叫就會改走這裡：記錄模式記下每次抽亂數，重播模式照序列回傳。
// 從 n 個中挑一個也記成 Int{0, n-1}，和 Emberstone 端的 ScriptedRng 對應。

#pragma once

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <iterator>
#include <numeric>
#include <random>
#include <stdexcept>
#include <string>
#include <type_traits>
#include <utility>
#include <vector>

namespace effolkronium
{
//! 一次抽亂數。
struct RngEvent
{
    enum class Kind
    {
        Int,
        Float,
        Shuffle
    };

    Kind kind = Kind::Int;
    long long lo = 0, hi = 0, value = 0;
    double flo = 0, fhi = 0, fvalue = 0;
    std::vector<std::size_t> perm;
};

class random_static
{
 public:
    using engine_type = std::mt19937_64;

    static engine_type& engine()
    {
        static engine_type e;
        return e;
    }

    static engine_type& get_engine()
    {
        return engine();
    }

    static void seed(std::uint64_t s)
    {
        engine().seed(s);
    }

    //! 記錄模式：以種子初始化並清空事件。
    static void StartRecord(std::uint64_t s)
    {
        State().replay = false;
        State().events.clear();
        State().next = 0;
        State().desync.clear();
        engine().seed(s);
    }

    //! 重播模式：之後的亂數照 events 依序回傳。
    static void StartReplay(std::vector<RngEvent> events)
    {
        State().replay = true;
        State().events = std::move(events);
        State().next = 0;
        State().desync.clear();
    }

    static const std::vector<RngEvent>& Events()
    {
        return State().events;
    }

    //! 重播時是否已經和序列不同步。不同步時不丟例外（RosettaStone 可能在 noexcept 的地方抽亂數），
    //! 改回傳安全的預設值並記下第一次不同步的原因，由橋接程式在每個行動後檢查。
    static bool Desynced()
    {
        return !State().desync.empty();
    }

    static const std::string& DesyncMessage()
    {
        return State().desync;
    }

    //! 閉區間 [from, to] 的整數；from > to 時對調（和 effolkronium 相同）。
    template <typename T>
    static std::enable_if_t<std::is_integral_v<T>, T> get(T from, T to)
    {
        if (from > to)
        {
            std::swap(from, to);
        }
        const auto lo = static_cast<long long>(from);
        const auto hi = static_cast<long long>(to);
        if (State().replay)
        {
            const RngEvent* e = Next(RngEvent::Kind::Int);
            if (e == nullptr || e->lo != lo || e->hi != hi)
            {
                Desync("int(" + std::to_string(lo) + ", " + std::to_string(hi) + ")");
                return from;
            }
            ++State().next;
            return static_cast<T>(e->value);
        }
        RngEvent e;
        e.kind = RngEvent::Kind::Int;
        e.lo = lo;
        e.hi = hi;
        e.value = std::uniform_int_distribution<long long>(lo, hi)(engine());
        State().events.push_back(e);
        return static_cast<T>(e.value);
    }

    template <typename T>
    static std::enable_if_t<std::is_floating_point_v<T>, T> get(T from, T to)
    {
        if (from > to)
        {
            std::swap(from, to);
        }
        const auto lo = static_cast<double>(from);
        const auto hi = static_cast<double>(to);
        if (State().replay)
        {
            const RngEvent* e = Next(RngEvent::Kind::Float);
            if (e == nullptr || e->flo != lo || e->fhi != hi)
            {
                Desync("float(" + std::to_string(lo) + ", " + std::to_string(hi) + ")");
                return from;
            }
            ++State().next;
            return static_cast<T>(e->fvalue);
        }
        RngEvent e;
        e.kind = RngEvent::Kind::Float;
        e.flo = lo;
        e.fhi = hi;
        e.fvalue = std::uniform_real_distribution<double>(lo, hi)(engine());
        State().events.push_back(e);
        return static_cast<T>(e.fvalue);
    }

    //! 從容器隨機挑一個；空容器回傳 end（不抽亂數）。
    template <typename Container>
    static auto get(Container& container) -> decltype(std::begin(container))
    {
        const auto n = static_cast<long long>(
            std::distance(std::begin(container), std::end(container)));
        if (n == 0)
        {
            return std::end(container);
        }
        const auto idx = get<long long>(0, n - 1);
        return std::next(std::begin(container), idx);
    }

    //! 洗牌：perm[i] 為洗牌後第 i 個位置放的原位置。
    template <typename RandomIt>
    static void shuffle(RandomIt first, RandomIt last)
    {
        const auto n = static_cast<std::size_t>(std::distance(first, last));
        std::vector<std::size_t> perm(n);
        std::iota(perm.begin(), perm.end(), std::size_t{ 0 });
        if (State().replay)
        {
            const RngEvent* e = Next(RngEvent::Kind::Shuffle);
            if (e == nullptr || e->perm.size() != n)
            {
                Desync("shuffle(" + std::to_string(n) + ")");
                return;
            }
            perm = e->perm;
            ++State().next;
        }
        else
        {
            std::shuffle(perm.begin(), perm.end(), engine());
            RngEvent e;
            e.kind = RngEvent::Kind::Shuffle;
            e.perm = perm;
            State().events.push_back(e);
        }
        using Value = typename std::iterator_traits<RandomIt>::value_type;
        std::vector<Value> original(first, last);
        for (std::size_t i = 0; i < n; ++i)
        {
            *(first + static_cast<std::ptrdiff_t>(i)) = original[perm[i]];
        }
    }

    template <typename Container>
    static void shuffle(Container& container)
    {
        shuffle(std::begin(container), std::end(container));
    }

 private:
    struct RngState
    {
        bool replay = false;
        std::vector<RngEvent> events;
        std::size_t next = 0;
        std::string desync;
    };

    static RngState& State()
    {
        static RngState state;
        return state;
    }

    //! 下一個事件（類型不符或已用完回傳 nullptr）。
    static const RngEvent* Next(RngEvent::Kind kind)
    {
        RngState& s = State();
        if (s.next >= s.events.size() || s.events[s.next].kind != kind)
        {
            return nullptr;
        }
        return &s.events[s.next];
    }

    static void Desync(const std::string& requested)
    {
        RngState& s = State();
        if (s.desync.empty())
        {
            s.desync = "亂數不同步：第 " + std::to_string(s.next) + " 個事件，要求 " + requested;
        }
    }
};
}  // namespace effolkronium
