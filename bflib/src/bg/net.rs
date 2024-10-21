use anyhow::Result;
use bfprotocols::perf::PerfStat;
use dcso3::perf::{HistStat, PerfStat as ApiPerfStat};
use netidx::{
    path::Path,
    publisher::{Publisher, UpdateBatch, Val},
};

struct PubHistStat {
    unit: Val,
    n: Val,
    mean: Val,
    twenty_five: Val,
    fifty: Val,
    ninety: Val,
    ninety_nine: Val,
    ninety_nine_nine: Val,
}

impl PubHistStat {
    fn new(publisher: &Publisher, base: &Path, stat: &HistStat) -> Result<Self> {
        let HistStat {
            name,
            unit,
            n,
            mean,
            twenty_five,
            fifty,
            ninety,
            ninety_nine,
            ninety_nine_nine,
        } = stat;
        let base = base.append(name);
        Ok(Self {
            unit: publisher.publish(base.append("unit"), unit)?,
            n: publisher.publish(base.append("n"), n)?,
            mean: publisher.publish(base.append("mean"), mean)?,
            twenty_five: publisher.publish(base.append("25th"), twenty_five)?,
            fifty: publisher.publish(base.append("50th"), fifty)?,
            ninety: publisher.publish(base.append("90th"), ninety)?,
            ninety_nine: publisher.publish(base.append("99th"), ninety_nine)?,
            ninety_nine_nine: publisher.publish(base.append("99.9th"), ninety_nine_nine)?,
        })
    }

    fn update(&self, batch: &mut UpdateBatch, stat: &HistStat) {
        let Self {
            unit: _,
            n,
            mean,
            twenty_five,
            fifty,
            ninety,
            ninety_nine,
            ninety_nine_nine,
        } = self;
        n.update_changed(batch, stat.n);
        mean.update_changed(batch, stat.mean);
        twenty_five.update_changed(batch, stat.twenty_five);
        fifty.update_changed(batch, stat.fifty);
        ninety.update_changed(batch, stat.ninety);
        ninety_nine.update_changed(batch, stat.ninety_nine);
        ninety_nine_nine.update_changed(batch, stat.ninety_nine_nine);
    }
}

struct PubPerf {
    players: Val,
    logistics_items: Val,
    frame: PubHistStat,
    timed_events: PubHistStat,
    slow_timed: PubHistStat,
    dcs_events: PubHistStat,
    dcs_hooks: PubHistStat,
    unit_positions: PubHistStat,
    player_positions: PubHistStat,
    ewr_tracks: PubHistStat,
    ewr_reports: PubHistStat,
    unit_culling: PubHistStat,
    remark_objectives: PubHistStat,
    update_jtac_contacts: PubHistStat,
    do_repairs: PubHistStat,
    spawn_queue: PubHistStat,
    spawn: PubHistStat,
    despawn: PubHistStat,
    advise_captured: PubHistStat,
    advise_capturable: PubHistStat,
    jtac_target_positions: PubHistStat,
    process_messages: PubHistStat,
    snapshot: PubHistStat,
    logistics: PubHistStat,
    logistics_distribute: PubHistStat,
    logistics_deliver: PubHistStat,
    logistics_sync_from: PubHistStat,
    logistics_sync_to: PubHistStat,
    get_position: PubHistStat,
    get_point: PubHistStat,
    get_velocity: PubHistStat,
    in_air: PubHistStat,
    get_ammo: PubHistStat,
    add_group: PubHistStat,
    add_static_object: PubHistStat,
    unit_is_exist: PubHistStat,
    unit_get_by_name: PubHistStat,
    unit_get_desc: PubHistStat,
    land_is_visible: PubHistStat,
    land_get_height: PubHistStat,
    timer_schedule_function: PubHistStat,
    timer_remove_function: PubHistStat,
    timer_get_time: PubHistStat,
    timer_get_abs_time: PubHistStat,
    timer_get_time0: PubHistStat,
}

impl PubPerf {
    fn new(
        publisher: &Publisher,
        base: &Path,
        players: usize,
        perf: &PerfStat,
        api_perf: &ApiPerfStat,
    ) -> Result<Self> {
        let PerfStat {
            frame,
            timed_events,
            slow_timed,
            dcs_events,
            dcs_hooks,
            unit_positions,
            player_positions,
            ewr_tracks,
            ewr_reports,
            unit_culling,
            remark_objectives,
            update_jtac_contacts,
            do_repairs,
            spawn_queue,
            spawn,
            despawn,
            advise_captured,
            advise_capturable,
            jtac_target_positions,
            process_messages,
            snapshot,
            logistics,
            logistics_distribute,
            logistics_deliver,
            logistics_sync_from,
            logistics_sync_to,
            logistics_items,
        } = perf;
        let ApiPerfStat {
            get_position,
            get_point,
            get_velocity,
            in_air,
            get_ammo,
            add_group,
            add_static_object,
            unit_is_exist,
            unit_get_by_name,
            unit_get_desc,
            land_is_visible,
            land_get_height,
            timer_schedule_function,
            timer_remove_function,
            timer_get_time,
            timer_get_abs_time,
            timer_get_time0,
        } = api_perf;
        Ok(Self {
            players: publisher.publish(base.append("players"), players)?,
            logistics_items: publisher.publish(base.append("logistics_items"), logistics_items)?,
            frame: PubHistStat::new(publisher, base, frame)?,
            timed_events: PubHistStat::new(publisher, base, timed_events)?,
            add_group: PubHistStat::new(publisher, base, add_group)?,
            add_static_object: PubHistStat::new(publisher, base, add_static_object)?,
            advise_capturable: PubHistStat::new(publisher, base, advise_capturable)?,
            advise_captured: PubHistStat::new(publisher, base, advise_captured)?,
            dcs_events: PubHistStat::new(publisher, base, dcs_events)?,
            dcs_hooks: PubHistStat::new(publisher, base, dcs_hooks)?,
            despawn: PubHistStat::new(publisher, base, despawn)?,
            do_repairs: PubHistStat::new(publisher, base, do_repairs)?,
            ewr_reports: PubHistStat::new(publisher, base, ewr_reports)?,
            ewr_tracks: PubHistStat::new(publisher, base, ewr_tracks)?,
            get_ammo: PubHistStat::new(publisher, base, get_ammo)?,
            get_point: PubHistStat::new(publisher, base, get_point)?,
            get_position: PubHistStat::new(publisher, base, get_position)?,
            get_velocity: PubHistStat::new(publisher, base, get_velocity)?,
            in_air: PubHistStat::new(publisher, base, in_air)?,
            jtac_target_positions: PubHistStat::new(publisher, base, jtac_target_positions)?,
            land_get_height: PubHistStat::new(publisher, base, land_get_height)?,
            land_is_visible: PubHistStat::new(publisher, base, land_is_visible)?,
            logistics: PubHistStat::new(publisher, base, logistics)?,
            logistics_deliver: PubHistStat::new(publisher, base, logistics_deliver)?,
            logistics_distribute: PubHistStat::new(publisher, base, logistics_distribute)?,
            logistics_sync_from: PubHistStat::new(publisher, base, logistics_sync_from)?,
            logistics_sync_to: PubHistStat::new(publisher, base, logistics_sync_to)?,
            player_positions: PubHistStat::new(publisher, base, player_positions)?,
            process_messages: PubHistStat::new(publisher, base, process_messages)?,
            remark_objectives: PubHistStat::new(publisher, base, remark_objectives)?,
            slow_timed: PubHistStat::new(publisher, base, slow_timed)?,
            snapshot: PubHistStat::new(publisher, base, snapshot)?,
            spawn: PubHistStat::new(publisher, base, spawn)?,
            spawn_queue: PubHistStat::new(publisher, base, spawn_queue)?,
            timer_get_abs_time: PubHistStat::new(publisher, base, timer_get_abs_time)?,
            timer_get_time: PubHistStat::new(publisher, base, timer_get_time)?,
            timer_get_time0: PubHistStat::new(publisher, base, timer_get_time0)?,
            timer_remove_function: PubHistStat::new(publisher, base, timer_remove_function)?,
            timer_schedule_function: PubHistStat::new(publisher, base, timer_schedule_function)?,
            unit_culling: PubHistStat::new(publisher, base, unit_culling)?,
            unit_get_by_name: PubHistStat::new(publisher, base, unit_get_by_name)?,
            unit_get_desc: PubHistStat::new(publisher, base, unit_get_desc)?,
            unit_is_exist: PubHistStat::new(publisher, base, unit_is_exist)?,
            unit_positions: PubHistStat::new(publisher, base, unit_positions)?,
            update_jtac_contacts: PubHistStat::new(publisher, base, update_jtac_contacts)?,
        })
    }

    fn update(
        &self,
        batch: &mut UpdateBatch,
        players: usize,
        perf: &PerfStat,
        api_perf: &ApiPerfStat,
    ) {
        let PerfStat {
            frame,
            timed_events,
            slow_timed,
            dcs_events,
            dcs_hooks,
            unit_positions,
            player_positions,
            ewr_tracks,
            ewr_reports,
            unit_culling,
            remark_objectives,
            update_jtac_contacts,
            do_repairs,
            spawn_queue,
            spawn,
            despawn,
            advise_captured,
            advise_capturable,
            jtac_target_positions,
            process_messages,
            snapshot,
            logistics,
            logistics_distribute,
            logistics_deliver,
            logistics_sync_from,
            logistics_sync_to,
            logistics_items,
        } = perf;
        let ApiPerfStat {
            get_position,
            get_point,
            get_velocity,
            in_air,
            get_ammo,
            add_group,
            add_static_object,
            unit_is_exist,
            unit_get_by_name,
            unit_get_desc,
            land_is_visible,
            land_get_height,
            timer_schedule_function,
            timer_remove_function,
            timer_get_time,
            timer_get_abs_time,
            timer_get_time0,
        } = api_perf;
        self.players.try_update_changed(batch, players);
        self.logistics_items.update_changed(batch, logistics_items);
        self.add_group.update(batch, add_group);
        self.add_static_object.update(batch, add_static_object);
        self.advise_capturable.update(batch, advise_capturable);
        self.advise_captured.update(batch, advise_captured);
        self.dcs_events.update(batch, dcs_events);
        self.dcs_hooks.update(batch, dcs_hooks);
        self.despawn.update(batch, despawn);
        self.do_repairs.update(batch, do_repairs);
        self.ewr_reports.update(batch, ewr_reports);
        self.ewr_tracks.update(batch, ewr_tracks);
        self.frame.update(batch, frame);
        self.get_ammo.update(batch, get_ammo);
        self.get_point.update(batch, get_point);
        self.get_position.update(batch, get_position);
        self.get_velocity.update(batch, get_velocity);
        self.in_air.update(batch, in_air);
        self.jtac_target_positions
            .update(batch, jtac_target_positions);
        self.land_get_height.update(batch, land_get_height);
        self.land_is_visible.update(batch, land_is_visible);
        self.logistics_deliver.update(batch, logistics_deliver);
        self.logistics_distribute
            .update(batch, logistics_distribute);
        self.logistics_sync_from.update(batch, logistics_sync_from);
        self.logistics_sync_to.update(batch, logistics_sync_to);
        self.logistics.update(batch, logistics);
        self.player_positions.update(batch, player_positions);
        self.process_messages.update(batch, process_messages);
        self.remark_objectives.update(batch, remark_objectives);
        self.slow_timed.update(batch, slow_timed);
        self.snapshot.update(batch, snapshot);
        self.spawn_queue.update(batch, spawn_queue);
        self.spawn.update(batch, spawn);
        self.timed_events.update(batch, timed_events);
        self.timer_get_abs_time.update(batch, timer_get_abs_time);
        self.timer_get_time0.update(batch, timer_get_time0);
        self.timer_get_time.update(batch, timer_get_time);
        self.timer_remove_function
            .update(batch, timer_remove_function);
        self.timer_schedule_function
            .update(batch, timer_schedule_function);
        self.unit_culling.update(batch, unit_culling);
        self.unit_get_by_name.update(batch, unit_get_by_name);
        self.unit_get_desc.update(batch, unit_get_desc);
        self.unit_is_exist.update(batch, unit_is_exist);
        self.unit_positions.update(batch, unit_positions);
        self.update_jtac_contacts
            .update(batch, update_jtac_contacts);
    }
}

pub struct T {
    publisher: Publisher,
    base: Path,
    perf: PubPerf,
}
