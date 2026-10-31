// Emberstone difftest: replacement for effolkronium::random_static (cases R1-R3).
// This directory goes first on the include path when building RosettaStone and the bridge,
// so every `Random::...` call in RosettaStone comes here: record mode logs each random draw,
// replay mode returns the logged results in order.
// Picking one of n is logged as Int{0, n-1}, matching ScriptedRng on the Emberstone side.
//
// This file must stay ASCII-only: RosettaStone is compiled without /utf-8, so MSVC reads it
// with the host code page (950 locally, 1252 on GitHub runners), and non-ASCII text (even in
// comments) can break parsing.

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
//! One random draw.
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

    //! Record mode: seed the engine and clear the events.
    static void StartRecord(std::uint64_t s)
    {
        State().replay = false;
        State().events.clear();
        State().next = 0;
        State().desync.clear();
        engine().seed(s);
    }

    //! Replay mode: later draws return the results in `events` in order.
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

    //! Whether replay has gone out of sync. No exception is thrown on desync (RosettaStone may
    //! draw inside noexcept code); a safe default is returned and the first reason is kept,
    //! and the bridge checks it after every action.
    static bool Desynced()
    {
        return !State().desync.empty();
    }

    static const std::string& DesyncMessage()
    {
        return State().desync;
    }

    //! Integer in the closed range [from, to]; swapped when from > to (same as effolkronium).
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

    //! Random element of a container; returns end for an empty container (no draw).
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

    //! Shuffle: after shuffling, position i holds the element originally at perm[i].
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

    //! The next event (nullptr if the kind differs or the sequence is used up).
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
            s.desync = "rng desync at event " + std::to_string(s.next) + ", requested " + requested;
        }
    }
};
}  // namespace effolkronium
