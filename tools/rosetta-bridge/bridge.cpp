// RosettaStone 橋接程式（差分測試用，case R1–R3、R5、I1–I5、S1）。
// 協定：stdin 每行一個 JSON 請求，stdout 每行一個 JSON 回應。
//   {"cmd":"version"} | {"cmd":"new",...} | {"cmd":"options"} | {"cmd":"act","action":{...}}
//   {"cmd":"snapshot"} | {"cmd":"events"} | {"cmd":"quit"}
// 成功：{"ok":true,...}；失敗：{"ok":false,"error":{"kind":...,"message":...}}

#include <Rosetta/PlayMode/Cards/Cards.hpp>
#include <Rosetta/PlayMode/Games/Game.hpp>
#include <Rosetta/PlayMode/Games/GameConfig.hpp>
#include <Rosetta/PlayMode/Models/HeroPower.hpp>
#include <Rosetta/PlayMode/Models/Minion.hpp>
#include <Rosetta/PlayMode/Models/Player.hpp>
#include <Rosetta/PlayMode/Tasks/PlayerTasks/AttackTask.hpp>
#include <Rosetta/PlayMode/Tasks/PlayerTasks/ChooseTask.hpp>
#include <Rosetta/PlayMode/Tasks/PlayerTasks/EndTurnTask.hpp>
#include <Rosetta/PlayMode/Tasks/PlayerTasks/HeroPowerTask.hpp>
#include <Rosetta/PlayMode/Tasks/PlayerTasks/PlayCardTask.hpp>
#include <Rosetta/PlayMode/Zones/DeckZone.hpp>
#include <Rosetta/PlayMode/Zones/FieldZone.hpp>
#include <Rosetta/PlayMode/Zones/HandZone.hpp>
#include <Rosetta/PlayMode/Zones/SecretZone.hpp>

#include <effolkronium/random.hpp>
#include <nlohmann/json.hpp>

#include <iostream>
#include <cstdlib>
#include <cstdio>
#include <exception>
#include <fstream>
#include <memory>
#include <sstream>
#include <string>
#include <utility>
#include <vector>

#ifndef ROSETTA_COMMIT
#define ROSETTA_COMMIT "unknown"
#endif
#ifndef BRIDGE_PATCHES
#define BRIDGE_PATCHES ""
#endif

using json = nlohmann::json;
using Rng = effolkronium::random_static;
using namespace RosettaStone;
using namespace RosettaStone::PlayMode;
using namespace RosettaStone::PlayMode::PlayerTasks;

namespace
{
struct BridgeError
{
    std::string kind;
    std::string message;
};

std::unique_ptr<Game> g_game;

Game& CurrentGame()
{
    if (!g_game)
    {
        throw BridgeError{ "bad_request", "還沒開局" };
    }
    return *g_game;
}

//! 重播不同步就作廢這局並回報（case R3、R4）。
void CheckDesync()
{
    if (Rng::Desynced())
    {
        g_game.reset();
        throw BridgeError{ "rng_desync", Rng::DesyncMessage() };
    }
}

CardClass ParseClass(const std::string& name)
{
    static const std::pair<const char*, CardClass> table[] = {
        { "DEMONHUNTER", CardClass::DEMONHUNTER }, { "DRUID", CardClass::DRUID },
        { "HUNTER", CardClass::HUNTER },           { "MAGE", CardClass::MAGE },
        { "PALADIN", CardClass::PALADIN },         { "PRIEST", CardClass::PRIEST },
        { "ROGUE", CardClass::ROGUE },             { "SHAMAN", CardClass::SHAMAN },
        { "WARLOCK", CardClass::WARLOCK },         { "WARRIOR", CardClass::WARRIOR },
    };
    for (const auto& [n, c] : table)
    {
        if (name == n)
        {
            return c;
        }
    }
    throw BridgeError{ "bad_request", "不明的職業 " + name };
}

Player* PlayerN(int n)
{
    return n == 1 ? CurrentGame().GetPlayer1() : CurrentGame().GetPlayer2();
}

int PlayerNumber(const Player* p)
{
    return p == CurrentGame().GetPlayer1() ? 1 : 2;
}

std::vector<Character*> Minions(Player* p)
{
    std::vector<Character*> out;
    for (auto* m : p->GetFieldZone()->GetAll())
    {
        out.push_back(dynamic_cast<Character*>(m));
    }
    return out;
}

//! 兩方所有角色：先手英雄、先手手下、後手英雄、後手手下。
std::vector<Character*> AllCharacters()
{
    std::vector<Character*> out;
    for (int n = 1; n <= 2; ++n)
    {
        out.push_back(PlayerN(n)->GetHero());
        for (Character* m : Minions(PlayerN(n)))
        {
            out.push_back(m);
        }
    }
    return out;
}

json TargetJson(const Character* c)
{
    if (c == nullptr)
    {
        return nullptr;
    }
    for (int n = 1; n <= 2; ++n)
    {
        if (c == PlayerN(n)->GetHero())
        {
            return { { "kind", "hero" }, { "player", n } };
        }
        const auto minions = Minions(PlayerN(n));
        for (std::size_t i = 0; i < minions.size(); ++i)
        {
            if (minions[i] == c)
            {
                return { { "kind", "minion" }, { "player", n }, { "index", i } };
            }
        }
    }
    throw BridgeError{ "internal", "找不到目標角色" };
}

Character* ResolveTarget(const json& t)
{
    if (t.is_null())
    {
        return nullptr;
    }
    Player* p = PlayerN(t.at("player").get<int>());
    if (t.at("kind") == "hero")
    {
        return p->GetHero();
    }
    const auto minions = Minions(p);
    const auto idx = t.at("index").get<std::size_t>();
    if (idx >= minions.size())
    {
        throw BridgeError{ "bad_request", "目標手下不存在" };
    }
    return minions[idx];
}

json MinionJson(Character* c)
{
    auto* m = dynamic_cast<Minion*>(c);
    return {
        { "card", c->card->id },
        { "attack", c->GetAttack() },
        { "health", c->GetHealth() },
        { "max_health", c->GetBaseHealth() },
        { "taunt", m != nullptr && m->HasTaunt() },
        { "divine_shield", m != nullptr && m->HasDivineShield() },
        { "frozen", c->IsFrozen() },
        { "stealth", m != nullptr && m->HasStealth() },
        { "windfury", c->HasWindfury() },
        { "attacks_this_turn", c->GetNumAttacksThisTurn() },
    };
}

json PlayerJson(Player* p)
{
    Hero* hero = p->GetHero();
    json weapon = nullptr;
    if (hero->HasWeapon())
    {
        weapon = { { "card", hero->weapon->card->id },
                   { "attack", hero->weapon->GetAttack() },
                   { "durability", hero->weapon->GetDurability() } };
    }
    json hand = json::array();
    for (Playable* c : p->GetHandZone()->GetAll())
    {
        hand.push_back(c->card->id);
    }
    json board = json::array();
    for (Character* m : Minions(p))
    {
        board.push_back(MinionJson(m));
    }
    json secrets = json::array();
    for (auto* s : p->GetSecretZone()->GetAll())
    {
        secrets.push_back(s->card->id);
    }
    return {
        { "hero",
          { { "card", hero->card->id },
            { "health", hero->GetHealth() },
            { "armor", hero->GetArmor() },
            { "attack", hero->GetAttack() },
            { "weapon", weapon },
            { "power_used", p->GetHeroPower().IsExhausted() } } },
        { "mana",
          { { "available", p->GetRemainingMana() },
            { "total", p->GetTotalMana() },
            { "overload_locked", p->GetOverloadLocked() },
            { "overload_owed", p->GetOverloadOwed() } } },
        { "hand", hand },
        { "board", board },
        { "deck_count", p->GetDeckZone()->GetCount() },
        { "secrets", secrets },
        // 玩家的 FATIGUE 標籤不會更新；疲勞累計記在英雄的 fatigue。
        { "fatigue", hero->fatigue },
    };
}

json Snapshot()
{
    Game& g = CurrentGame();
    const bool over = g.state == State::COMPLETE;
    json winner = nullptr;
    if (over)
    {
        if (g.GetPlayer1()->playState == PlayState::WON)
        {
            winner = 1;
        }
        else if (g.GetPlayer2()->playState == PlayState::WON)
        {
            winner = 2;
        }
    }
    return {
        { "turn", g.GetTurn() },
        { "current_player", PlayerNumber(g.GetCurrentPlayer()) },
        { "game_over", over },
        { "winner", winner },
        { "players", json::array({ PlayerJson(g.GetPlayer1()), PlayerJson(g.GetPlayer2()) }) },
    };
}

//! 目前玩家的所有合法行動（case I5）。
json Options()
{
    Game& g = CurrentGame();
    json out = json::array();
    if (g.state == State::COMPLETE)
    {
        return out;
    }
    Player* p = g.GetCurrentPlayer();
    if (p->choice && p->choice->choiceType == ChoiceType::GENERAL)
    {
        for (std::size_t i = 0; i < p->choice->choices.size(); ++i)
        {
            out.push_back({ { "type", "choose" }, { "index", i } });
        }
        return out;
    }

    const auto characters = AllCharacters();
    const auto hand = p->GetHandZone()->GetAll();
    for (std::size_t i = 0; i < hand.size(); ++i)
    {
        Playable* card = hand[i];
        const CardType type = card->card->GetCardType();
        if (!card->IsPlayableByPlayer() || type == CardType::LOCATION)
        {
            continue;
        }
        const bool minion = type == CardType::MINION;
        if (minion && p->GetFieldZone()->IsFull())
        {
            continue;
        }
        std::vector<int> chooses = { 0 };
        if (card->HasChooseOne())
        {
            chooses = { 1, 2 };
        }
        for (const int co : chooses)
        {
            if (!card->IsPlayableByCardReq(co))
            {
                continue;
            }
            std::vector<Character*> targets;
            if (card->IsValidPlayTarget(nullptr, co))
            {
                targets.push_back(nullptr);
            }
            for (Character* t : characters)
            {
                if (card->IsValidPlayTarget(t, co))
                {
                    targets.push_back(t);
                }
            }
            for (Character* t : targets)
            {
                if (minion)
                {
                    const int count = p->GetFieldZone()->GetCount();
                    for (int pos = 0; pos <= count; ++pos)
                    {
                        out.push_back({ { "type", "play" },
                                        { "hand", i },
                                        { "pos", pos },
                                        { "target", TargetJson(t) },
                                        { "choose_one", co } });
                    }
                }
                else
                {
                    out.push_back({ { "type", "play" },
                                    { "hand", i },
                                    { "pos", nullptr },
                                    { "target", TargetJson(t) },
                                    { "choose_one", co } });
                }
            }
        }
    }

    std::vector<Character*> attackers = Minions(p);
    attackers.push_back(p->GetHero());
    for (Character* a : attackers)
    {
        if (a == nullptr || !a->CanAttack())
        {
            continue;
        }
        for (Character* t : a->GetValidAttackTargets(p->opponent))
        {
            // GetValidAttackTargets 沒排除「剛出場的衝刺手下打英雄」，要再用 IsValidAttackTarget 過濾，
            // 否則這個行動會被列出，執行時卻什麼都不做。
            if (!a->IsValidAttackTarget(p->opponent, t))
            {
                continue;
            }
            out.push_back({ { "type", "attack" },
                            { "attacker", TargetJson(a) },
                            { "target", TargetJson(t) } });
        }
    }

    HeroPower& power = p->GetHeroPower();
    if (power.IsPlayableByPlayer() && power.IsPlayableByCardReq())
    {
        if (power.IsValidPlayTarget(nullptr))
        {
            out.push_back({ { "type", "power" }, { "target", nullptr } });
        }
        for (Character* t : characters)
        {
            if (power.IsValidPlayTarget(t))
            {
                out.push_back({ { "type", "power" }, { "target", TargetJson(t) } });
            }
        }
    }

    out.push_back({ { "type", "end" } });
    return out;
}

//! 執行行動；不在合法行動清單裡就拒絕（case I4，狀態不變）。
void Act(const json& action)
{
    bool legal = false;
    for (const json& o : Options())
    {
        if (o == action)
        {
            legal = true;
            break;
        }
    }
    if (!legal)
    {
        throw BridgeError{ "illegal_action", "不合法的行動：" + action.dump() };
    }

    Game& g = CurrentGame();
    Player* p = g.GetCurrentPlayer();
    const std::string type = action.at("type");
    if (type == "play")
    {
        Playable* card = p->GetHandZone()->GetAll().at(action.at("hand").get<std::size_t>());
        const int pos = action.at("pos").is_null() ? -1 : action.at("pos").get<int>();
        g.Process(p, PlayCardTask(card, ResolveTarget(action.at("target")), pos,
                                  action.at("choose_one").get<int>()));
    }
    else if (type == "attack")
    {
        g.Process(p, AttackTask(ResolveTarget(action.at("attacker")),
                                ResolveTarget(action.at("target"))));
    }
    else if (type == "power")
    {
        g.Process(p, HeroPowerTask(ResolveTarget(action.at("target"))));
    }
    else if (type == "choose")
    {
        const int entity = p->choice->choices.at(action.at("index").get<std::size_t>());
        g.Process(p, ChooseTask(std::vector<int>{ entity }));
    }
    else
    {
        g.Process(p, EndTurnTask());
    }
    CheckDesync();
}

std::vector<effolkronium::RngEvent> ParseEvents(const json& events)
{
    using Kind = effolkronium::RngEvent::Kind;
    std::vector<effolkronium::RngEvent> out;
    for (const json& e : events)
    {
        effolkronium::RngEvent ev;
        const std::string kind = e.at("kind");
        if (kind == "int")
        {
            ev.kind = Kind::Int;
            ev.lo = e.at("lo").get<long long>();
            ev.hi = e.at("hi").get<long long>();
            ev.value = e.at("value").get<long long>();
        }
        else if (kind == "float")
        {
            ev.kind = Kind::Float;
            ev.flo = e.at("lo").get<double>();
            ev.fhi = e.at("hi").get<double>();
            ev.fvalue = e.at("value").get<double>();
        }
        else if (kind == "shuffle")
        {
            ev.kind = Kind::Shuffle;
            ev.perm = e.at("perm").get<std::vector<std::size_t>>();
        }
        else
        {
            throw BridgeError{ "bad_request", "不明的亂數事件 " + kind };
        }
        out.push_back(std::move(ev));
    }
    return out;
}

json EventsJson()
{
    using Kind = effolkronium::RngEvent::Kind;
    json out = json::array();
    for (const auto& e : Rng::Events())
    {
        switch (e.kind)
        {
            case Kind::Int:
                out.push_back({ { "kind", "int" }, { "lo", e.lo }, { "hi", e.hi }, { "value", e.value } });
                break;
            case Kind::Float:
                out.push_back({ { "kind", "float" }, { "lo", e.flo }, { "hi", e.fhi }, { "value", e.fvalue } });
                break;
            case Kind::Shuffle:
                out.push_back({ { "kind", "shuffle" }, { "n", e.perm.size() }, { "perm", e.perm } });
                break;
        }
    }
    return out;
}

//! 開局（case I1、I2）：照抽牌順序、不洗牌、不換牌、先手固定為 1 號。
void NewGame(const json& req)
{
    GameConfig config;
    config.formatType = FormatType::WILD;
    config.startPlayer = PlayerType::PLAYER1;
    config.player1Class = ParseClass(req.at("p1").at("class"));
    config.player2Class = ParseClass(req.at("p2").at("class"));
    const auto fill = [](const json& deck, std::array<Card*, START_DECK_SIZE>& out) {
        if (deck.size() != START_DECK_SIZE)
        {
            throw BridgeError{ "bad_request", "牌組要剛好 30 張" };
        }
        for (std::size_t i = 0; i < deck.size(); ++i)
        {
            const std::string id = deck[i];
            Card* card = Cards::FindCardByID(id);
            if (card == nullptr || card->id.empty())
            {
                throw BridgeError{ "bad_request", "RosettaStone 沒有這張卡：" + id };
            }
            out[i] = card;
        }
    };
    fill(req.at("p1").at("deck"), config.player1Deck);
    fill(req.at("p2").at("deck"), config.player2Deck);
    config.doShuffle = false;
    config.skipMulligan = true;
    config.autoRun = true;

    g_game.reset();
    const json& rng = req.at("rng");
    if (rng.at("mode") == "record")
    {
        Rng::StartRecord(rng.at("seed").get<std::uint64_t>());
    }
    else
    {
        Rng::StartReplay(ParseEvents(rng.at("events")));
    }
    g_game = std::make_unique<Game>(config);
    g_game->Start();
    CheckDesync();
}

json Version()
{
    json patches = json::array();
    std::stringstream ss(BRIDGE_PATCHES);
    std::string name;
    while (std::getline(ss, name, ';'))
    {
        if (!name.empty())
        {
            patches.push_back(name);
        }
    }
    return { { "ok", true }, { "rosetta_commit", ROSETTA_COMMIT }, { "patches", patches } };
}

json Handle(const json& req)
{
    const std::string cmd = req.at("cmd");
    if (cmd == "version")
    {
        return Version();
    }
    if (cmd == "new")
    {
        NewGame(req);
        return { { "ok", true }, { "snapshot", Snapshot() } };
    }
    if (cmd == "options")
    {
        return { { "ok", true }, { "options", Options() } };
    }
    if (cmd == "act")
    {
        Act(req.at("action"));
        return { { "ok", true }, { "snapshot", Snapshot() } };
    }
    if (cmd == "snapshot")
    {
        return { { "ok", true }, { "snapshot", Snapshot() } };
    }
    if (cmd == "events")
    {
        return { { "ok", true }, { "events", EventsJson() } };
    }
    throw BridgeError{ "bad_request", "不明的指令 " + cmd };
}

json ErrorJson(const std::string& kind, const std::string& message)
{
    return { { "ok", false }, { "error", { { "kind", kind }, { "message", message } } } };
}
}  // namespace

int main()
{
    // 除錯用：程式被 std::terminate 結束時，把原因寫到 stderr。
    std::set_terminate([] {
        try
        {
            if (const auto e = std::current_exception())
            {
                std::rethrow_exception(e);
            }
            std::fprintf(stderr, "terminate: 沒有進行中的例外\n");
        }
        catch (const std::exception& ex)
        {
            std::fprintf(stderr, "terminate: %s\n", ex.what());
        }
        catch (...)
        {
            std::fprintf(stderr, "terminate: 不明例外\n");
        }
        std::fflush(stderr);
        std::abort();
    });

    // RosettaStone 若在 std::cout 印東西會打亂協定，所以把 std::cout 導到空處，回應另外寫。
    std::ostream out(std::cout.rdbuf());
    std::stringstream sink;
    std::cout.rdbuf(sink.rdbuf());

    // 除錯用：設定 EMBERSTONE_BRIDGE_LOG 時把每個請求附加寫入該檔，方便重現當機。
    std::ofstream log;
    if (const char* path = std::getenv("EMBERSTONE_BRIDGE_LOG"))
    {
        log.open(path, std::ios::app);
    }

    Cards::GetInstance();
    std::string line;
    while (std::getline(std::cin, line))
    {
        if (line.empty())
        {
            continue;
        }
        if (log.is_open())
        {
            log << line << '\n';
            log.flush();
        }
        json response;
        try
        {
            const json req = json::parse(line);
            if (req.at("cmd") == "quit")
            {
                break;
            }
            response = Handle(req);
        }
        catch (const BridgeError& e)
        {
            response = ErrorJson(e.kind, e.message);
        }
        catch (const json::exception& e)
        {
            response = ErrorJson("bad_request", e.what());
        }
        catch (const std::exception& e)
        {
            response = ErrorJson("engine_exception", e.what());
        }
        catch (...)
        {
            // 以 /EHa 編譯時，RosettaStone 內部的記憶體存取錯誤也會到這裡；這局的狀態已不可信，直接作廢。
            g_game.reset();
            response = ErrorJson("engine_crash", "RosettaStone 內部錯誤（這局已作廢）");
        }
        sink.str("");
        out << response.dump(-1, ' ', false, json::error_handler_t::replace) << '\n';
        out.flush();
    }
    return 0;
}
